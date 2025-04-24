#!/bin/bash
echo 'Starting ffplay...'
ffplay -loglevel warning -rtsp_transport tcp -max_delay 200000 -fflags nobuffer -flags low_delay -framedrop -infbuf -sync ext -vf "setpts=0.5*PTS" -analyzeduration 500000 -probesize 500000 -x 800 -y 600 -window_title "Olympus Camera Stream" -autoexit rtsp://192.168.0.10:5555 2>/dev/null &
sleep 1
pgrep -f 'ffplay' | head -1
