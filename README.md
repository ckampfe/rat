rat
===

# what

a reimagining of https://rasterbator.net/, inspired by the source: https://github.com/supertobi/rasterbator-ng

# dev

`$ cargo web start --release`

# build

`$ cargo web deploy --release`

In `target/deploy`:
`$ wasm-opt -Os -o rat.wasm rat.wasm`


# todo

- [ ] styling/UX
- [ ] zip all files for download, "download all" link
- [x] svg backend
- [ ] way to make the thing not block while it renders, if possible
- [ ] any kind of perf optimization at all
- [ ] `wee_alloc` (https://rustwasm.github.io/book/game-of-life/code-size.html)
