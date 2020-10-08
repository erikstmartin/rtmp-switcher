#!/usr/bin/env bash
gst_version=$(gst-launch-1.0 --version | grep version | tr -s ' ' '\n' | tail -1)
git clone -b $gst_version --depth 1 git://anongit.freedesktop.org/git/gstreamer/gst-plugins-bad
cd gst-plugins-bad

./autogen.sh --disable-gtk-doc --noconfigure
NVENCODE_CFLAGS="-I/usr/local/cuda/include" ./configure --with-cuda-prefix="/usr/local/cuda"

plugins=( "sys/nvenc" "sys/nvdec" "ext/fdkaac" )
for p in "${plugins[@]}"
do
  cd $p
  make
  make install
  cd ../..
done

cp /usr/local/lib/gstreamer-1.0/libgst*.so /usr/lib/x86_64-linux-gnu/gstreamer-1.0/
