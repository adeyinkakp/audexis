//
//  CloudPlaybackButton.swift
//  audexis
//
//  Created by Kp on 10/22/26.
//

import SwiftUI

struct CloudPlaybackButton: View {
  @Environment(CloudLibrary.self) private var library
  var prominent = false

  private var symbol: String {
    if library.playback.error != nil { return "arrow.clockwise" }
  
    return library.playback.isPlaying ? "pause.fill" : "play.fill"
  }

  var body: some View {
    Button {
      if library.playback.error != nil {
        library.retryPlayback()
      } else {
        library.playback.toggle()
      }
    } label: {
      Group {
        if library.playback.isLoading && library.playback.error == nil {
          ProgressView()
        } else {
          Image(systemName: symbol).font(prominent ? .system(size: 58) : .title3)
        }
      }
      .frame(width: prominent ? 58 : 44, height: prominent ? 58 : 44)
    }
    .disabled(
      library.playback.track == nil || (library.playback.isLoading && library.playback.error == nil)
    )
    .accessibilityLabel(
      library.playback.error != nil
        ? "Retry playback" : library.playback.isPlaying ? "Pause" : "Play")
  }
}
