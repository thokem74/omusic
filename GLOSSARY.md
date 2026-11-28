# Omusic

A terminal music player for a personal collection of local MP3 files.

## Language

**Collection**:
The user's existing local MP3 files available to browse and play.
_Avoid_: Catalog

**Track**:
A single piece of music represented by a file in the collection.
_Avoid_: Song

**Album**:
A named group of tracks intended to be listened to in a defined order.

**Album artist**:
The artist credit for an album as a whole, including a shared credit such as Various Artists for a compilation. It can differ from the artist credits of individual tracks.

**Various Artists**:
A shared album credit for multiple artists. It is also the fallback display credit when an album has no unambiguous album artist or shared track artist.

**Track artist**:
The artist credit for an individual track, including any credited collaborators or guests.

**Compilation**:
An album bringing together tracks credited to different artists under a shared album identity.

**Appears on**:
The section of an artist's browser page containing albums to which that artist contributes through track credits without being credited as the album artist.

**Queue**:
The ordered list of tracks selected for playback. Playing an album replaces this list; adding tracks or albums to it preserves the existing entries.
_Avoid_: Playlist

**Session**:
The browsing position, volume, and queue remembered between launches. Restoring a session does not start playback.
