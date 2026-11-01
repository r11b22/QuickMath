# Built-in spelling dictionaries

Hunspell dictionaries compiled into QuickMath (see `src/spelling/languages.rs`),
taken unchanged from [wooorm/dictionaries](https://github.com/wooorm/dictionaries)
at commit `8cfea406b505e4d7df52d5a19bce525df98c54ab`.

- `en_US`: English (United States), from SCOWL, version 2020.12.07.
  Licence in `en_US.license` (MIT/BSD-style).
- `nl_NL`: Dutch, from OpenTaal. Licence in `nl_NL.license`
  (Revised BSD or CC BY 3.0).

To update one, replace its `.aff` and `.dic` files (and licence) with a newer
copy. Files must be UTF-8. More languages do not need to be built in: users
can drop any Hunspell `.aff`/`.dic` pair into their dictionary folder.
