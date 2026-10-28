//
//  Search.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct SearchPage: View {
  @Environment(CloudLibrary.self) private var library
  @State private var query = ""
  private var search: String { query.trimmingCharacters(in: .whitespacesAndNewlines) }
  private var matches: [CloudTrack] {
    library.snapshot.tracks.filter {
      "\($0.title) \($0.artist) \($0.album)".localizedCaseInsensitiveContains(search)
    }
  }

  var body: some View {
    List {
      Section { CloudLibraryStatus().listRowBackground(Color.clear) }
      if search.isEmpty {
        Section {
          VStack(alignment: .leading, spacing: 8) {
            Text("Find something        .").font(.title.bold())
            Text("Search songs, artists, and albums in your library.").foregroundStyle(
              .secondary)
          }.padding(.vertical, 12).listRowBackground(Color.clear)
        }
        Section("Browse albums") {
          CloudCollectionGrid(
            collections: Array(
              MusicCollection.albums(in: library.snapshot.tracks).prefix(12))
          )
          .padding(.vertical, 8).listRowBackground(Color.clear).listRowSeparator(.hidden)
        }
      } else if matches.isEmpty {
        ContentUnavailableView.search(text: search).listRowBackground(Color.clear)
      } else {
        let albums = MusicCollection.albums(in: matches).filter {
          $0.title.localizedCaseInsensitiveContains(search)
        }
        if !albums.isEmpty {
          Section("Albums") {
            CloudCollectionGrid(collections: Array(albums.prefix(6)))
              .padding(.vertical, 8).listRowBackground(Color.clear).listRowSeparator(
                .hidden)
          }
        }
        Section("Songs · \(matches.count)") {
          ForEach(matches) { CloudTrackRow(track: $0, context: matches) }
        }
      }
    }
    .listStyle(.plain).scrollContentBackground(.hidden)
    .searchable(text: $query, prompt: "Songs, artists, albums")
    .autocorrectionDisabled()
  }
}
