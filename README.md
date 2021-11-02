rat
===

# what

a reimagining of https://rasterbator.net/, inspired by the source: https://github.com/supertobi/rasterbator-ng

# dev

```
# dev profile
$ trunk serve

# release profile
$ trunk serve --release
```

# build

`$ trunk build --release`

Compress the contents of `dist` into a zip file and upload

# todo

- [x] styling/UX
- [x] zip all files for download, "download all" link
- [x] svg backend
- [x] allow user to control min/max percentages for dot sizes
- [ ] way to make the thing not block while it renders, if possible
- [ ] any kind of perf optimization at all
- [x] investigate converting to `web-sys` from `stdweb`
- [ ] `wee_alloc` (https://rustwasm.github.io/book/game-of-life/code-size.html)
- [ ] gzip the wasm bundle (seems to be an issue with Amplify)
