//
//  CloudCollectionActions.swift
//  audexis
//
//  Created by Kp on 10/26/26.
//

import SwiftUI

struct CloudCollectionActions: View {
  @Environment(CloudLibrary.self) private var library
  let tracks: [CloudTrack]
  var body: some View {
    HStack(spacing: 12) {
      Button {
        if let first = tracks.first { library.play(first, among: tracks) }
      } label: {
        Label("Play", systemImage: "play.fill").frame(maxWidth: .infinity).padding(.vertical, 8)
      }
      .buttonStyle(.borderedProminent)
      Button {
        let shuffled = tracks.shuffled()
        if let first = shuffled.first { library.play(first, among: shuffled) }
      } label: {
        Label("Shuffle", systemImage: "shuffle").frame(maxWidth: .infinity).padding(.vertical, 8)
      }
      .buttonStyle(.bordered)
    }
    .buttonBorderShape(.roundedRectangle(radius: 12))
    .disabled(tracks.isEmpty)
  }
}
