//
//  CloudPlayerLyrics.swift
//  audexis
//
//  Created by Kp on 10/26/26.
//

import Foundation
import SwiftUI

struct CloudPlayerLyrics: View {
  @Environment(CloudLibrary.self) private var library
  let track: CloudTrack
  @State private var lyrics = PlayerLyrics()
  @State private var following = true

  private var active: Int? {
    lyrics.lines.lastIndex { $0.seconds <= library.playback.position }
  }

  var body: some View {
    Group {
      if !lyrics.lines.isEmpty {
        ScrollViewReader { proxy in
          ScrollView {
            LazyVStack(alignment: .leading, spacing: 12) {
              ForEach(Array(lyrics.lines.enumerated()), id: \.offset) { index, line in
                if line.text.isEmpty {
                  if index == active,
                    lyrics.breakEnd(at: index).map({ $0 - line.seconds >= 5 })
                      ?? true
                  {
                    CloudLyricsWaiting(
                      position: library.playback.position,
                      start: line.seconds,
                      end: lyrics.breakEnd(at: index)
                    )
                    .id(index)
                  }
                } else {
                  Button {
                    library.playback.seek(line.seconds)
                    following = true
                    withAnimation { proxy.scrollTo(index, anchor: .center) }
                  } label: {
                    Text(line.text)
                      .font(.title.bold())
                      .foregroundStyle(
                        index == active
                          ? AudexisStyle.accent
                          : AudexisStyle.content_faint
                      )
                      .frame(maxWidth: .infinity, alignment: .leading)
                  }
                  .buttonStyle(.plain)
                  .accessibilityLabel(
                    "\(line.text), seek to \(Int(line.seconds)) seconds"
                  )
                  .id(index)
                }
              }
            }
            .padding(.horizontal, 28).padding(.vertical, 48)
          }
          .simultaneousGesture(DragGesture().onChanged { _ in following = false })
          .onChange(of: active) {
            if following, let active {
              withAnimation(.easeInOut(duration: 0.3)) {
                proxy.scrollTo(active, anchor: .center)
              }
            }
          }
          .onAppear { if let active { proxy.scrollTo(active, anchor: .center) } }
          .overlay(alignment: .bottomTrailing) {
            if !following {
              Button {
                following = true
                if let active {
                  withAnimation { proxy.scrollTo(active, anchor: .center) }
                }
              } label: {
                Label("Follow lyrics", systemImage: "location.fill")
                  .font(.caption.weight(.semibold)).padding(12)
                  .background(.regularMaterial, in: Capsule())
              }
              .padding(12)
            }
          }
        }
      } else if !lyrics.plain.isEmpty {
        ScrollView {
          Text(lyrics.plain).font(.title2.weight(.semibold)).lineSpacing(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .textSelection(.enabled).padding(28)
        }
      } else {
        ContentUnavailableView(
          "No lyrics available", systemImage: "quote.bubble",
          description: Text(
            "Lyrics will appear here when they’re included in the song’s synced metadata."
          ))
      }
    }
    .onAppear { lyrics = PlayerLyrics(track: track) }
  }
}

private struct PlayerLyrics {
  struct Line {
    let seconds: Double
    let text: String
  }
  var lines: [Line] = []
  var plain = ""

  init() {}

  init(track: CloudTrack) {
    let synced = track.metadata.tags["lyrics"]?.joined(separator: "\n") ?? ""
    let unsynced = track.metadata.tags["unsyncedLyrics"]?.joined(separator: "\n") ?? ""
    let text =
      synced.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? unsynced : synced
    guard let pattern = try? NSRegularExpression(pattern: #"\[(\d+):(\d{2})(?:\.(\d{1,3}))?\]"#)
    else {
      plain = text
      return
    }
    let offsetPattern = try? NSRegularExpression(pattern: #"(?m)^\[offset:([+-]?\d+)\]\s*$"#)
    let fullRange = NSRange(text.startIndex..., in: text)
    var offset: Double = 0
    if let match = offsetPattern?.firstMatch(in: text, range: fullRange),
      let range = Range(match.range(at: 1), in: text), let milliseconds = Double(text[range])
    {
      offset = milliseconds / 1000
    }
    for raw in text.components(separatedBy: .newlines) {
      let matches = pattern.matches(in: raw, range: NSRange(raw.startIndex..., in: raw))
      guard let last = matches.last, let end = Range(last.range, in: raw)?.upperBound else {
        continue
      }
      let content = String(raw[end...]).trimmingCharacters(in: .whitespaces)
      for match in matches {
        guard let minutesRange = Range(match.range(at: 1), in: raw),
          let secondsRange = Range(match.range(at: 2), in: raw),
          let minutes = Double(raw[minutesRange]),
          let seconds = Double(raw[secondsRange]), seconds < 60
        else { continue }
        let fraction =
          Range(match.range(at: 3), in: raw).flatMap { Double("0." + raw[$0]) } ?? 0
        lines.append(
          Line(seconds: max(0, minutes * 60 + seconds + fraction + offset), text: content)
        )
      }
    }
    lines.sort { $0.seconds < $1.seconds }
    var grouped: [Line] = []
    for line in lines {
      if line.text.isEmpty && grouped.last?.text.isEmpty == true { continue }
      grouped.append(line)
    }
    lines = grouped
    if let first = lines.first, first.seconds > 0 {
      if first.text.isEmpty {
        lines[0] = Line(seconds: 0, text: "")
      } else {
        lines.insert(Line(seconds: 0, text: ""), at: 0)
      }
    }
    if !lines.contains(where: { !$0.text.isEmpty }) { lines = [] }
    if lines.isEmpty { plain = text.trimmingCharacters(in: .whitespacesAndNewlines) }
  }

  func breakEnd(at index: Int) -> Double? {
    lines.dropFirst(index + 1).first(where: { !$0.text.isEmpty })?.seconds
  }
}
