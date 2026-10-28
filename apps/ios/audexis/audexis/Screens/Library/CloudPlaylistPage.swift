//
//  CloudPlaylistPage.swift
//  audexis
//
//  Created by Kp on 10/22/26.
//

import SwiftUI

struct CloudPlaylistPage: View {
  @Environment(CloudLibrary.self) private var library
  let playlist: CloudPlaylist
  var downloadedOnly = false

  var body: some View {
    let current = library.snapshot.playlists.first { $0.id == playlist.id } ?? playlist
    let ids = current.track_ids.filter {
      !downloadedOnly || library.snapshot.downloads[$0] != nil
    }
    CloudTrackCollectionPage(
      title: current.name, subtitle: downloadedOnly ? "Downloaded playlist" : "Playlist",
      trackIDs: ids)
  }
}
