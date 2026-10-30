//
//  HomePage.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct HomePage: View {
  @Environment(CloudLibrary.self) private var library
  private var albums: [MusicCollection] { MusicCollection.albums(in: library.snapshot.tracks) }

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 28) {
        CloudLibraryStatus()
        if !library.snapshot.tracks.isEmpty {
          VStack(alignment: .leading, spacing: 6) {
            Text("Your music.").font(.largeTitle.bold())
            Text("\(library.snapshot.tracks.count) Songs")
              .foregroundStyle(.secondary)
          }
          HStack(spacing: 12) {
            NavigationLink {
              LibraryPage(downloadedOnly: true)
            } label: {
              Label("Downloaded", systemImage: "arrow.down.circle.fill")
                .font(.subheadline.weight(.semibold)).padding(14).frame(
                  maxWidth: .infinity
                )
                .background(
                  AudexisStyle.card, in: RoundedRectangle(cornerRadius: 14), )
            }
            NavigationLink {
              LibraryPage()
            } label: {
              Label("All music", systemImage: "music.note.list")
                .font(.subheadline.weight(.semibold)).padding(14).frame(
                  maxWidth: .infinity
                )
                .background(
                  AudexisStyle.card, in: RoundedRectangle(cornerRadius: 14), )
            }
          }
          FavoritesPlaylistLink(_chevHidden: true, _downloadOnly: false)
          if let track = library.playback.track {
            VStack(alignment: .leading, spacing: 14) {
              Text("Now playing").font(.title2.bold())
              CloudTrackRow(track: track, onPlay: { library.playback.resume() })
            }
          }
          VStack(alignment: .leading, spacing: 16) {
            Text("From your library").font(.title2.bold())
            CloudCollectionGrid(collections: Array(albums.prefix(12)))
          }
          if !library.snapshot.playlists.isEmpty {
            VStack(alignment: .leading, spacing: 16) {
              Text("Made by you").font(.title2.bold())
              ForEach(library.snapshot.playlists.prefix(6)) { playlist in
                NavigationLink {
                  CloudPlaylistPage(playlist: playlist)
                } label: {
                  HStack(spacing: 14) {
                    Image(systemName: "music.note.list").font(.title2).frame(
                      width: 44, height: 44,
                    )
                    .background(
                      AudexisStyle.primary_soft,
                      in: RoundedRectangle(cornerRadius: 10), )
                    VStack(alignment: .leading, spacing: 4) {
                      Text(playlist.name).font(.headline)
                      Text("\(playlist.track_ids.count) songs").font(.caption)
                        .foregroundStyle(.secondary)
                    }
                    Spacer()
                    Image(systemName: "chevron.right").font(.caption)
                      .foregroundStyle(.secondary)
                  }
                  .padding(12).background(
                    AudexisStyle.card, in: RoundedRectangle(cornerRadius: 16), )
                }
                .buttonStyle(.plain)
              }
            }
          }
        } else if !library.isLoading {
          ContentUnavailableView(
            "Unavailable", systemImage: "music.note.house",
            description: Text(
              "Upload songs from Audexis Desktop. Pull down to refresh"
            ), )
        }
      }
      .padding(20).frame(maxWidth: 1100).frame(maxWidth: .infinity)
    }
    .refreshable {
      await library.refresh()
      await library.importDownloads()
    }
  }
}
