//
//  LibraryPage.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct LibraryPage: View {
  private enum Category: String, CaseIterable {
    case songs = "Songs"
    case albums = "Albums"
    case artists = "Artists"
    case playlists = "Playlists"
  }
  @Environment(CloudLibrary.self) private var library
  @State private var search = ""
  @State var downloadedOnly = false
  @State private var category = Category.songs
  @State private var showQueue = false

  private var tracks: [CloudTrack] {
    library.snapshot.tracks.filter {
      (!downloadedOnly || library.snapshot.downloads[$0.id] != nil)
        && (search.isEmpty
          || "\($0.title) \($0.artist) \($0.album)".localizedCaseInsensitiveContains(
            search))
    }
  }

  var body: some View {
    List {
      Section {
        CloudLibraryStatus().listRowBackground(Color.clear)
        ScrollView(.horizontal) {
          HStack(spacing: 8) {
            ForEach(Category.allCases, id: \.self) { item in
              Button {
                category = item
              } label: {
                Text(item.rawValue).font(.subheadline.weight(.semibold))
                  .padding(.horizontal, 16).padding(.vertical, 10)
                  .background(
                    category == item
                      ? AudexisStyle.primary_soft : AudexisStyle.card,
                    in: Capsule(), )
              }
              .buttonStyle(.plain)
              .accessibilityAddTraits(category == item ? .isSelected : [])
            }
          }
        }
        .scrollIndicators(.hidden).listRowBackground(Color.clear)
        Toggle(isOn: $downloadedOnly) {
          Label("Downloaded", systemImage: "arrow.down.circle")
        }
        .font(.subheadline).listRowBackground(Color.clear)
      }
      if !tracks.isEmpty {
        if category == .songs {
          Section {
            CloudCollectionActions(tracks: tracks).listRowBackground(Color.clear)
            ForEach(tracks) { CloudTrackRow(track: $0, context: tracks) }
          } header: {
            Text("\(tracks.count) songs")
          }
        } else if category == .albums || category == .artists {
          Section {
            CloudCollectionGrid(
              collections: category == .albums
                ? MusicCollection.albums(in: tracks)
                : MusicCollection.artists(in: tracks)
            )
            .padding(.vertical, 8).listRowBackground(Color.clear).listRowSeparator(
              .hidden)
          }
        }
      } else if !library.isLoading && category != .playlists {
        ContentUnavailableView(
          downloadedOnly ? "No downloaded songs" : "No songs found",
          systemImage: "music.note",
          description: Text(
            downloadedOnly
              ? "Download music to keep it with you offline."
              : "Try another search or add music from Audexis Desktop."),
        )
        .listRowBackground(Color.clear)
      }
      if category == .playlists {
        let showFavorites =
          (search.isEmpty || "Favorites".localizedCaseInsensitiveContains(search))
          && (!downloadedOnly
            || library.snapshot.favorites.contains { library.snapshot.downloads[$0.id] != nil })
        let playlists = library.snapshot.playlists.filter { playlist in
          (search.isEmpty || playlist.name.localizedCaseInsensitiveContains(search))
            && (!downloadedOnly
              || playlist.track_ids.contains { library.snapshot.downloads[$0] != nil })
        }
        if showFavorites {
          FavoritesPlaylistLink(_chevHidden: true, _downloadOnly: downloadedOnly)
            .listRowBackground(Color.clear).listRowSeparator(.hidden)
        }
        if playlists.isEmpty && !showFavorites {
          ContentUnavailableView(
            "No playlists yet", systemImage: "music.note.list",
            description: Text("Playlists you create on Desktop will sync here."),
          )
          .listRowBackground(Color.clear)
        }
        ForEach(playlists) { playlist in
          NavigationLink {
            CloudPlaylistPage(playlist: playlist, downloadedOnly: downloadedOnly)
          } label: {
            HStack(spacing: 14) {
              Image(systemName: "music.note.list").font(.title2).foregroundStyle(
                AudexisStyle.primary
              )
              .frame(width: 52, height: 52).background(
                AudexisStyle.primary_soft, in: RoundedRectangle(cornerRadius: 12), )
              VStack(alignment: .leading, spacing: 4) {
                Text(playlist.name).font(.headline)
                Text("\(playlist.track_ids.count) songs").font(.caption)
                  .foregroundStyle(.secondary)
              }
            }.padding(.vertical, 4)
          }
          .listRowBackground(AudexisStyle.card)
        }
      }
    }
    .listStyle(.plain)
    .scrollContentBackground(.hidden)
    .navigationTitle(downloadedOnly ? "Downloaded" : "Library")

    .sheet(isPresented: $showQueue) { CloudPlayerView(panel: .queue) }
    .searchable(text: $search, prompt: "Your music")
    .refreshable {
      await library.refresh()
      await library.importDownloads()
    }
  }
}
