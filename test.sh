#!/bin/bash

http -v POST http://localhost:3000/chromecast/cast ip=192.168.8.106 url=http://192.168.8.103:3000/media_show?file=//?/C:/Source/Rust/casterson/test_data/big_buck_bunny.mp4

# http -v POST http://localhost:3000/chromecast/cast ip=192.168.8.106 "url=http://192.168.8.103:3000/media_show?file=\\\\\\?\\C:\\Source\\Rust\\casterson\\test_data\\big_buck_bunny.mp4"