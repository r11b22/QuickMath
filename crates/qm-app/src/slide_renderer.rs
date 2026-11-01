//! The background thread that opens a slide deck and renders its pages, so
//! rendering never holds up typing.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use qm_core::{Project, SlideImage};

/// What the render thread reports back.
#[derive(Debug)]
pub enum RenderEvent {
    Opened {
        page_count: usize,
    },
    /// `width` is the width that was asked for; the image may be a pixel narrower.
    Rendered {
        page: usize,
        width: u32,
        image: SlideImage,
    },
    /// The deck cannot be shown. The thread stops after sending this.
    Failed(String),
}

#[derive(Debug, Clone, Copy)]
struct RenderRequest {
    page: usize,
    width: u32,
}

/// One render thread per open deck. Dropping it stops the thread once its
/// current page is done.
pub struct SlideRenderer {
    requests: Sender<RenderRequest>,
    events: Receiver<RenderEvent>,
}

impl SlideRenderer {
    /// Starts opening `deck` on a new thread. `ctx` is repainted whenever an
    /// event arrives, so the UI does not have to poll on a timer.
    pub fn start(project: &Project, deck: &Path, ctx: &egui::Context) -> SlideRenderer {
        let (request_sender, request_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();
        let worker = RenderWorker {
            project: project.clone(),
            deck: deck.to_path_buf(),
            requests: request_receiver,
            events: event_sender,
            ctx: ctx.clone(),
        };
        std::thread::spawn(move || worker.run());
        SlideRenderer {
            requests: request_sender,
            events: event_receiver,
        }
    }

    /// Asks for `page` rendered `width` pixels wide. The newest request is
    /// rendered first, so flipping quickly shows the page the user stopped on.
    pub fn request(&self, page: usize, width: u32) {
        // A send only fails once the thread has stopped, which `poll` reports.
        let _ = self.requests.send(RenderRequest { page, width });
    }

    /// The events that arrived since the last call.
    pub fn poll(&self) -> Vec<RenderEvent> {
        let mut events = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // After `Failed` the thread has stopped as expected. Otherwise
                    // it can only have stopped by panicking inside the PDF renderer.
                    if matches!(events.last(), Some(RenderEvent::Failed(_))) {
                        break;
                    }
                    events.push(RenderEvent::Failed(
                        "the slides renderer stopped unexpectedly".to_string(),
                    ));
                    break;
                }
            }
        }
        events
    }
}

struct RenderWorker {
    project: Project,
    deck: PathBuf,
    requests: Receiver<RenderRequest>,
    events: Sender<RenderEvent>,
    ctx: egui::Context,
}

impl RenderWorker {
    fn run(self) {
        let deck = match self.project.open_slides(&self.deck) {
            Ok(deck) => deck,
            Err(error) => {
                self.send(RenderEvent::Failed(error.to_string()));
                return;
            }
        };
        self.send(RenderEvent::Opened {
            page_count: deck.page_count(),
        });
        let mut pending = Vec::new();
        while let Some(request) = self.next_request(&mut pending) {
            let event = match deck.render_page(request.page, request.width) {
                Ok(image) => RenderEvent::Rendered {
                    page: request.page,
                    width: request.width,
                    image,
                },
                Err(error) => {
                    self.send(RenderEvent::Failed(error.to_string()));
                    return;
                }
            };
            self.send(event);
        }
    }

    /// The newest waiting request, blocking until one arrives. `None` once
    /// the renderer has been dropped.
    fn next_request(&self, pending: &mut Vec<RenderRequest>) -> Option<RenderRequest> {
        pending.extend(self.requests.try_iter());
        pending.pop().or_else(|| self.requests.recv().ok())
    }

    fn send(&self, event: RenderEvent) {
        // Fails only when the renderer was dropped; nobody is waiting for the event then.
        let _ = self.events.send(event);
        self.ctx.request_repaint();
    }
}
