#!/bin/bash

http -v POST http://localhost:3000/chromecast/cast ip=192.168.8.106 url=http://localhost:3000/media_show?{%22file%22:%22//?/C:/Source/Rust/casterson/test_data/big_buck_bunny.mp4%22}

# http://localhost:3000/media_show?{%22file%22:%22//?/C:/Source/Rust/casterson/test_data/big_buck_bunny.mp4%22,%22encode_opts%22:{%22seek_seconds%22:120}}
