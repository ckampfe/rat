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
- [x] zip all files for download, "download all" link
- [x] svg backend
- [x] allow user to control min/max percentages for dot sizes
- [ ] way to make the thing not block while it renders, if possible
- [ ] any kind of perf optimization at all
- [x] investigate converting to `web-sys` from `stdweb`
- [ ] `wee_alloc` (https://rustwasm.github.io/book/game-of-life/code-size.html)
