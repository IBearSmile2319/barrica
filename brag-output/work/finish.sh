#!/bin/sh
# Render final: cuadros → brag.mp4 (1920×1080, 30 fps, H.264 + AAC), con la portada como cuadro 0.
set -eu
cd "$(dirname "$0")"
POSTER=f0147.png # 4,9 s: «Barrica 0.2 · Ahora con su propio Wine», ya asentado
node render.js
cp "frames/$POSTER" ../brag-poster.png
cp "frames/$POSTER" frames/f0000.png # sustituye el cuadro 0: misma duración, mismo sonido
ffmpeg -hide_banner -loglevel error -y -framerate 30 -i frames/f%04d.png -i soundtrack.wav \
    -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p -c:a aac -b:a 192k -shortest -movflags +faststart ../brag.mp4
ffmpeg -hide_banner -loglevel error -y -i ../brag-poster.png -q:v 2 ../brag.jpg
rm ../brag-poster.png
ffprobe -v error -show_entries format=duration:stream=codec_name,width,height -of default=nw=1 ../brag.mp4
