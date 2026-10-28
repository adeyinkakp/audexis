//
//  CloudTrackActions.swift
//  audexis
//
//  Created by Kp on 10/22/26.
//

import SwiftUI

struct CloudTrackActions: View {
  @Environment(CloudLibrary.self) private var library
  let track: CloudTrack
  var context: [CloudTrack] = []
  var onPlay: (() -> Void)? = nil
  var onRemoveFromQueue: (() -> Void)? = nil

  var body: some View {
    Button("Play", systemImage: "play.fill") {
      if let onPlay {
        onPlay()
      } else if context.isEmpty && library.playback.track?.id == track.id {
        if library.playback.error != nil {
          library.retryPlayback()
        } else {
          library.playback.resume()
        }
      } else {
        library.play(track, among: context.isEmpty ? [track] : context)
      }
    }
    Button("Play next", systemImage: "text.line.first.and.arrowtriangle.forward") {
      library.enqueue(track, next: true)
    }
    Button("Add to queue", systemImage: "text.badge.plus") {
      library.enqueue(track, next: false)
    }
    Divider()
    if library.snapshot.downloads[track.id] != nil {
      Button("Remove download", systemImage: "trash", role: .destructive) {
        library.removeDownload(track)
      }
    } else if library.downloads[track.id] != nil {
      Button("Cancel download", systemImage: "xmark") {
        library.cancelDownload(track)
      }
    } else {
      Button("Download", systemImage: "arrow.down.circle") {
        library.download(track)
      }
    }
    if let onRemoveFromQueue {
      Divider()
      Button("Remove from queue", systemImage: "text.badge.minus", role: .destructive) {
        onRemoveFromQueue()
      }
    }
  }
}
