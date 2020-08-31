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
