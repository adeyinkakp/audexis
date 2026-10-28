//
//  CloudPlayerControls.swift
//  audexis
//
//  Created by Kp on 10/26/26.
//

import SwiftUI

struct CloudPlayerControls: View {
  @Environment(CloudLibrary.self) private var library

  var body: some View {
    VStack(spacing: 12) {
      CloudPlayerSeekBar(
        position: library.playback.position, duration: library.playback.duration
      ) {
        library.playback.seek($0)
      }
      .id(library.playback.track?.id)
      HStack(spacing: 0) {
        Spacer(minLength: 0)
        Button {
          library.playback.onPrevious?()
        } label: {
          Image(systemName: "backward.end.fill").frame(width: 48, height: 48)
        }
        .accessibilityLabel("Previous song")
        Spacer(minLength: 12)
        CloudPlaybackButton(prominent: true)
        Spacer(minLength: 12)
        Button {
          library.advance()
        } label: {
          Image(systemName: "forward.end.fill").frame(width: 48, height: 48)
        }
        .accessibilityLabel("Next song")
        Spacer(minLength: 0)
      }
      .font(.title).buttonStyle(.plain)
      if let error = library.playback.error {
        Text(error).font(.caption).foregroundStyle(AudexisStyle.destructive)
          .lineLimit(3).accessibilityLabel("Playback error: \(error)")
      }
    }
  }

}
