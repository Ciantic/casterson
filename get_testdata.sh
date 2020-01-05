#!/bin/bash

if [ ! -f ./test_data/big_buck_bunny.mp4 ]; then
    wget -O ./test_data/big_buck_bunny.mp4 http://commondatastorage.googleapis.com/gtv-videos-bucket/big_buck_bunny_1080p.mp4
fi

echo "1
00:00:00,498 --> 00:00:02,826
- This is an example subtitle
second line here. ÄÖäö.

2
00:00:02,826 --> 00:00:06,384
- In one line

3
00:00:06,384 --> 00:00:09,428
- And something else too?
- Okay." > ./test_data/big_buck_bunny.srt