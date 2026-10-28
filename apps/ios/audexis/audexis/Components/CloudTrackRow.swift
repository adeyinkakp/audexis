//
//  CloudTrackRow.swift
//  audexis
//
//  Created by Kp on 10/22/26.
//

import SwiftUI

struct CloudTrackRow: View {
  @Environment(CloudLibrary.self) private var library
  let track: CloudTrack
  var context: [CloudTrack] = []
  var onPlay: (() -> Void)? = nil
  var isCurrent: Bool? = nil
  var onRemoveFromQueue: (() -> Void)? = nil

  private var actions: some View {
    CloudTrackActions(
      track: track, context: context, onPlay: onPlay, onRemoveFromQueue: onRemoveFromQueue)
  }

  var body: some View {
    HStack(spacing: 12) {
      Button {
        if let onPlay {
          onPlay()
        } else {
          library.play(track, among: context.isEmpty ? [track] : context)
        }
      } label: {
        HStack(spacing: 12) {
          CloudArtwork(track: track)
          CloudTrackDetails(
            track: track, highlighted: isCurrent ?? (library.playback.track?.id == track.id))
          Spacer(minLength: 0)
        }
        .contentShape(Rectangle())
      }
      .buttonStyle(.plain)
      CloudDownloadIndicator(track: track)
      Menu {
        actions
      } label: {
        Image(systemName: "ellipsis")
          .frame(width: 44, height: 44)
          .contentShape(Rectangle())
      }
      .accessibilityLabel("Options for \(track.title)")
    }
    .contextMenu { actions }
    .padding(.vertical, 4)
    .listRowBackground(AudexisStyle.card)
  }
}
