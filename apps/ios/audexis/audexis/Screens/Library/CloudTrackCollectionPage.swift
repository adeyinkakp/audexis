//
//  CloudTrackCollectionPage.swift
//  audexis
//
//  Created by Kp on 10/26/26.
//

import SwiftUI

struct CloudTrackCollectionPage: View {
  @Environment(CloudLibrary.self) private var library
  let title: String
  let subtitle: String
  let trackIDs: [String]

  private var tracks: [CloudTrack] {
    let byID = Dictionary(
      library.snapshot.tracks.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first }, )
    return trackIDs.compactMap { byID[$0] }
  }

  var body: some View {
    List {
      Section {
        HStack(alignment: .center, spacing: 18) {
          if let first = tracks.first { CloudArtwork(track: first, size: 96) }
          VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.title2.bold())
            Text(subtitle).font(.subheadline).foregroundStyle(.secondary)
            Text("\(tracks.count) songs").font(.caption).foregroundStyle(.secondary)
          }
        }
        .listRowBackground(Color.clear)
        CloudCollectionActions(tracks: tracks).listRowBackground(Color.clear)
      }
      Section {
        ForEach(Array(tracks.enumerated()), id: \.offset) { _, track in
          CloudTrackRow(track: track, context: tracks)
        }
      }
    }
    .listStyle(.plain)
    .scrollContentBackground(.hidden)
    .background(AudexisStyle.background)
    .navigationTitle(title).navigationBarTitleDisplayMode(.inline)
    .toolbar {
      ToolbarItem(placement: .topBarTrailing) {
        Button {
          for track in tracks { library.download(track) }
        } label: {
          Image(systemName: "arrow.down.circle")
        }
        .disabled(tracks.isEmpty).accessibilityLabel("Download collection")
      }
    }
    .overlay {
      if tracks.isEmpty { ContentUnavailableView("No songs", systemImage: "music.note") }
    }
  }
}
