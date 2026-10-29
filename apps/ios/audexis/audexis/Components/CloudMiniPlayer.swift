//
//  CloudMiniPlayer.swift
//  audexis
//
//  Created by Kp on 10/22/26.
//

import SwiftUI

struct CloudMiniPlayer: View {
  @Environment(CloudLibrary.self) private var library
  @State private var expanded = false

  var body: some View {
    HStack(spacing: 12) {
      Button {
        expanded = true
      } label: {
        HStack(spacing: 10) {
          if let track = library.playback.track {
            CloudArtwork(track: track, size: 40)
            CloudTrackDetails(
              track: track, titleFont: .subheadline.weight(.semibold),
              subtitle: library.playback.error == nil ? nil : "Playback unavailable · Tap to retry",
            )
          } else {
            Text("Nothing is playing").font(.subheadline.weight(.semibold))
          }
          Spacer(minLength: 0)
        }
        .contentShape(Rectangle())
      }
      .buttonStyle(.plain)
      CloudPlaybackButton()
      Button {
        library.advance()
      } label: {
        Image(systemName: "forward.end.fill").frame(width: 44, height: 44)
      }
      .disabled(library.playback.track == nil)
      .accessibilityLabel("Next song")
    }
    .padding(.horizontal, 12)
    .padding(.vertical, 8)
    .frame(maxWidth: .infinity)
    .contextMenu {
      if let track = library.playback.track { CloudTrackActions(track: track) }
    }
    .sheet(isPresented: $expanded) { CloudPlayerView() }
  }
}
