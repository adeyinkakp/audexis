//
//  CloudPlayerQueue.swift
//  audexis
//
//  Created by Kp on 10/26/26.
//

import SwiftUI

struct CloudPlayerQueue: View {
  @Environment(CloudLibrary.self) private var library

  var body: some View {
    let tracks = Dictionary(
      library.snapshot.tracks.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
    List {
      if let current = library.snapshot.current,
        library.snapshot.queue.indices.contains(current),
        let track = tracks[library.snapshot.queue[current]]
      {
        Section("Playing Now") {
          CloudTrackRow(track: track, onPlay: { library.selectQueue(current) }, isCurrent: true)
        }
      }
      Section {
        let first = (library.snapshot.current ?? -1) + 1
        let upcoming = library.snapshot.queue.enumerated().filter { $0.offset >= first }
        if upcoming.isEmpty {
          Text("No upcoming songs").foregroundStyle(.secondary)
            .listRowBackground(Color.clear)
        }
        ForEach(upcoming, id: \.offset) { index, id in
          if let track = tracks[id] {
            CloudTrackRow(
              track: track, onPlay: { library.selectQueue(index) }, isCurrent: false,
              onRemoveFromQueue: { library.removeQueue(index) }
            )
            .swipeActions {
              Button("Remove", role: .destructive) { library.removeQueue(index) }
            }
          }
        }
      } header: {
        HStack {
          Text("Up Next")
          Spacer()
          if let current = library.snapshot.current, library.snapshot.queue.count > current + 1 {
            Button("Clear") {
              for index in stride(
                from: library.snapshot.queue.count - 1, through: current + 1, by: -1)
              {
                library.removeQueue(index)
              }
            }
            .textCase(nil)
            .accessibilityLabel("Clear upcoming songs")
          }
        }
      }
    }
    .listStyle(.plain)
    .scrollContentBackground(.hidden)
  }
}
