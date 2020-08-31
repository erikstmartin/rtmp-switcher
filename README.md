# RTMP Switcher
Status: (Pre-Alpha, UNSTABLE)

This project will allow you to simulate a production video switcher with virtual streams. Allowing multiple streamers to broadcast
to separate RTMP endpoints and seamlessly switch between them as well as pre-recorded videos.

Future features planned:
- Transitions
- Failover to pre-recorded versions of talks.
- Hardware encoding.
- Multiple outputs (restream).
- Lower 3rds.
- Picture in Picture.
- Tooling to deploy alongside [Owncast](https://github.com/owncast/owncast).

Packages: (Not all of these are required yet. If you have all of these though. You shouldn't have trouble using any documented elements.)
I use pacman, so your package names may differ.
```
gstreamer
gstreamer-vaapi
qt-gstreamer
gst-editing-services
gst-libav
gst-plugin-gtk
gst-plugins-bad
gst-plugins-bad-libs
gst-plugins-base
gst-plugins-base-libs
gst-plugins-good
gst-plugins-ugly
gstreamer
gstreamer-vaapi
qt-gstreamer
```
## License
Copyright 2020 Erik St. Martin

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
