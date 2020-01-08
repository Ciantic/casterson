#!/bin/bash

http -v POST http://localhost:3000/chromecast/cast ip=192.168.8.106 'url=http://192.168.8.103:3000/media_show?{%22file%22:%22//?/C:/Source/Rust/casterson/test_data/big_buck_bunny.mp4%22}'

# http -v POST http://localhost:3000/chromecast/status ip=192.168.8.106
# http -v POST http://localhost:3000/chromecast/stop ip=192.168.8.106

# http://localhost:3000/media_show?{%22file%22:%22//?/C:/Source/Rust/casterson/test_data/big_buck_bunny.mp4%22,%22encode_opts%22:{%22seek_seconds%22:120}}
