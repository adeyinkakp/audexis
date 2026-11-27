# TODO and Annoyances

Anything I plan on doing in the future in no paticular order. Anything with a
question mark at the end is a maybe or potentially in the far future.

- Switch from SQLX migrations it breaks to easily
- Add logging before release
- Keyboard navugation isn't great
- Finish docs and maybe make it more technical
- UI is kinda bland
- UNdo and redo support. Make an Action struct that has any changes a user
  (For example user changes playlist name that has the before change and after
  change stored)
  Batch editng will probably not be supported due to complexity. I would say
  for now just say show a modal saying these changes cannot being undone.
  And also reasonable limit of history for potential high memory usage.
  IE mulltiple image cover changes.
- Make playlist tracks dragable like queue
- Playback stops when there is an audio problem. IE like a decoder problem and
  that cause queue to be cleared.
- Missing files is a real pain when there are multiple files it's too annoying
  to relink or delete records.
- Add lyrics to search index
- Now playing bar on desktop doesnt actually float over anything. It just has
  its own spot at the bottom
- Cloud sync page is kinda useless
- Editing playlists is unintuitive and ugly
- Create a settign where audexis is allowed to organize library based
  on metadata
- custom artist pages
- Make rewind playlist auto update. For exmaple if i create a rewind playlist
  based on the current year and my rewind histroy changes like my number one
  played song goes becomes my number 3 the current behavior is that
  the rewind playtlist stays the same
- Compress image covers?
- Plugin support?????
