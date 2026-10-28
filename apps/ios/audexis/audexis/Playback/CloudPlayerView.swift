//
//  CloudPlayerView.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import SwiftUI

struct CloudPlayerView: View {
  enum Panel { case artwork, lyrics, queue }
  @Environment(CloudLibrary.self) private var library
  @Environment(\.dismiss) private var dismiss
  @State var panel: Panel = .artwork
  @ScaledMetric(relativeTo: .title2) private var headingSpace: CGFloat = 96

  var body: some View {
    GeometryReader { geometry in
      let wide =
        geometry.size.width >= 620
        || (geometry.size.width >= 500 && geometry.size.width > geometry.size.height)
      VStack(spacing: 12) {
        header
        if let track = library.playback.track {
          if wide {
            HStack(spacing: 0) {
              VStack(spacing: 10) {
                artwork(track)
                controls
                panelButtons(wide: true)
              }
              .frame(maxWidth: .infinity)
              Divider().padding(.horizontal, 12)
              VStack(alignment: .leading, spacing: 8) {
                Text(panel == .lyrics ? "Lyrics" : "Playing Next")
                  .font(.title2.bold()).padding(.horizontal, 20)
                details(track, lyrics: panel == .lyrics)
              }
              .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
          } else {
            if panel == .artwork {
              artwork(track)
            } else {
              songHeading(track, compact: true).fixedSize(horizontal: false, vertical: true)
              details(track, lyrics: panel == .lyrics)
            }
            controls
            panelButtons()
          }
        } else {
          ContentUnavailableView(
            "Nothing is playing", systemImage: "music.note",
            description: Text("Choose a song from your library.")
          )
          .frame(maxHeight: .infinity)
        }
      }
      .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
    .background(AudexisStyle.background.ignoresSafeArea())
    .foregroundStyle(AudexisStyle.foreground)
    .tint(AudexisStyle.primary)
    .presentationDetents([.large])
    .presentationDragIndicator(.visible)
  }

  private var controls: some View {
    CloudPlayerControls()
      .padding(.horizontal, 20)
      .fixedSize(horizontal: false, vertical: true)
  }

  private func panelButtons(wide: Bool = false) -> some View {
    HStack(spacing: 48) {
      panelButton(.lyrics, symbol: "quote.bubble", label: "Lyrics", wide: wide)
      panelButton(.queue, symbol: "list.bullet", label: "Queue", wide: wide)
    }
    .frame(maxWidth: .infinity).padding(.bottom, 8)
  }

  @ViewBuilder private func details(_ track: CloudTrack, lyrics: Bool) -> some View {
    if lyrics {
      CloudPlayerLyrics(track: track).id("\(track.id):\(track.revision)")
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    } else {
      CloudPlayerQueue().frame(maxWidth: .infinity, maxHeight: .infinity)
    }
  }

  private func artwork(_ track: CloudTrack) -> some View {
    GeometryReader { space in
      let size = max(0, min(space.size.width - 40, space.size.height - headingSpace - 20, 420))
      VStack(spacing: 14) {
        Spacer(minLength: 0)
        CloudArtwork(track: track, size: size)
          .shadow(color: .black.opacity(0.16), radius: 16, y: 8)
          .contextMenu { CloudTrackActions(track: track) }
        songHeading(track, compact: false)
          .fixedSize(horizontal: false, vertical: true).layoutPriority(1)
        Spacer(minLength: 0)
      }
      .padding(.horizontal, 20)
      .frame(width: space.size.width, height: space.size.height)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
  }

  private var header: some View {
    HStack {
      Button {
        dismiss()
      } label: {
        Image(systemName: "chevron.down").frame(width: 44, height: 44)
      }
      .accessibilityLabel("Close Now Playing")
      Spacer()
      Text(panel == .queue ? "Playing Next" : panel == .lyrics ? "Lyrics" : "Now Playing")
        .font(.subheadline.weight(.semibold))
      Spacer()
      if let track = library.playback.track {
        Menu {
          CloudTrackActions(track: track)
        } label: {
          Image(systemName: "ellipsis").frame(width: 44, height: 44)
        }
        .accessibilityLabel("Options for \(track.title)")
      } else {
        Color.clear.frame(width: 44, height: 44)
      }
    }
    .padding(.horizontal, 12)
    .padding(.top, 12)
  }

  private func songHeading(_ track: CloudTrack, compact: Bool) -> some View {
    HStack(spacing: 12) {
      if compact { CloudArtwork(track: track, size: 52) }
      VStack(alignment: .leading, spacing: 4) {
        Text(track.title).font(compact ? .headline : .title2.bold()).lineLimit(2)
        Text(track.artist).font(compact ? .subheadline : .title3)
          .foregroundStyle(.secondary).lineLimit(1)
      }
      Spacer(minLength: 0)
      CloudDownloadIndicator(track: track)
    }
    .padding(.horizontal, compact ? 24 : 0)
  }

  private func panelButton(_ target: Panel, symbol: String, label: String, wide: Bool) -> some View
  {
    let selected = panel == target || (wide && panel == .artwork && target == .queue)
    return Button {
      withAnimation(.easeInOut(duration: 0.2)) {
        panel =
          wide
          ? (target == .lyrics && !selected ? .lyrics : .queue) : (selected ? .artwork : target)
      }
    } label: {
      Image(systemName: symbol).font(.title3)
        .frame(width: 54, height: 44)
        .background(
          selected ? AudexisStyle.primary_soft : .clear, in: RoundedRectangle(cornerRadius: 12))
    }
    .disabled(library.playback.track == nil)
    .accessibilityLabel(label)
    .accessibilityAddTraits(selected ? .isSelected : [])
  }
}
