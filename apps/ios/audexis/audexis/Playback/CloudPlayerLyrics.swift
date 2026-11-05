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
    @State private var lastInteraction: Date?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @ScaledMetric(relativeTo: .title) private var lyricSize: CGFloat = 34

    private var active: Int? {
        lyrics.lines.lastIndex { $0.seconds <= library.playback.position }
    }

    var body: some View {
        Group {
            if !lyrics.lines.isEmpty {
                GeometryReader { geometry in
                    ScrollViewReader { proxy in
                        ScrollView {
                            LazyVStack(alignment: .leading, spacing: 24) {
                                ForEach(Array(lyrics.lines.enumerated()), id: \.offset) {
                                    index, line in
                                    if line.text.isEmpty {
                                        if index == active,
                                            lyrics.breakEnd(at: index).map({
                                                $0 - line.seconds >= 5
                                            }) ?? true
                                        {
                                            CloudLyricsWaiting(
                                                position: library.playback.position,
                                                start: line.seconds,
                                                end: lyrics.breakEnd(at: index),
                                            )
                                            .foregroundStyle(.white)
                                            .id(index)
                                        }
                                    } else {
                                        let distance = active.map { abs(index - $0) } ?? 1
                                        let highlighted = index == active
                                        Button {
                                            library.playback.seek(line.seconds)
                                            following = true
                                            follow(index, using: proxy)
                                        } label: {
                                            Text(line.text)
                                                .font(
                                                    .system(
                                                        size: lyricSize, weight: .bold,
                                                        design: .rounded, )
                                                )
                                                .fixedSize(horizontal: false, vertical: true)
                                                .lineSpacing(3)
                                                .foregroundStyle(.white)
                                                .opacity(
                                                    highlighted
                                                        ? 1
                                                        : following
                                                            ? (distance < 2 ? 0.55 : 0.28) : 0.8
                                                )
                                                .blur(
                                                    radius: reduceMotion || !following
                                                        ? 0
                                                        : CGFloat(max(0, min(distance - 1, 3)))
                                                            * 0.65
                                                )
                                                .scaleEffect(
                                                    reduceMotion || highlighted ? 1 : 0.96,
                                                    anchor: .leading,
                                                )
                                                .frame(
                                                    maxWidth: .infinity, minHeight: 44,
                                                    alignment: .leading,
                                                )
                                                .contentShape(Rectangle())
                                        }
                                        .buttonStyle(.plain)
                                        .animation(
                                            reduceMotion ? nil : .easeInOut(duration: 0.45),
                                            value: distance,
                                        )
                                        .animation(
                                            reduceMotion ? nil : .easeInOut(duration: 0.45),
                                            value: highlighted,
                                        )
                                        .animation(
                                            reduceMotion ? nil : .easeInOut(duration: 0.45),
                                            value: following,
                                        )
                                        .accessibilityLabel(
                                            "\(line.text), seek to \(Int(line.seconds)) seconds"
                                        )
                                        .accessibilityAddTraits(highlighted ? .isSelected : [])
                                        .id(index)
                                    }
                                }
                            }
                            .padding(.horizontal, 28)
                            .padding(.top, geometry.size.height * 0.32)
                            .padding(.bottom, geometry.size.height * 0.65)
                        }
                        .scrollIndicators(.hidden)
                        .mask {
                            LinearGradient(
                                stops: [
                                    .init(color: .clear, location: 0),
                                    .init(color: .black, location: 0.08),
                                    .init(color: .black, location: 0.88),
                                    .init(color: .clear, location: 1),
                                ], startPoint: .top, endPoint: .bottom,
                            )
                        }
                        .simultaneousGesture(
                            DragGesture().onChanged { _ in
                                following = false
                                lastInteraction = Date()
                            }.onEnded { _ in lastInteraction = Date() }
                        )
                        .onChange(of: active) {
                            if following, let active { follow(active, using: proxy) }
                        }
                        .onAppear {
                            if let active {
                                proxy.scrollTo(active, anchor: UnitPoint(x: 0.5, y: 0.35))
                            }
                        }
                        .task(id: lastInteraction) {
                            guard lastInteraction != nil else { return }
                            do { try await Task.sleep(for: .seconds(6)) } catch { return }
                            following = true
                            if let active { follow(active, using: proxy) }
                        }
                        .overlay(alignment: .bottomTrailing) {
                            if !following {
                                Button {
                                    following = true
                                    if let active { follow(active, using: proxy) }
                                } label: {
                                    Label("Follow lyrics", systemImage: "location.fill")
                                        .font(.caption.weight(.semibold)).padding(12)
                                        .foregroundStyle(.white)
                                        .background(.ultraThinMaterial, in: Capsule())
                                }
                                .padding(12)
                            }
                        }
                    }
                }
            } else if !lyrics.plain.isEmpty {
                ScrollView {
                    Text(lyrics.plain)
                        .font(.system(size: lyricSize, weight: .bold, design: .rounded))
                        .foregroundStyle(.white.opacity(0.85)).lineSpacing(12)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .textSelection(.enabled).padding(28)
                }
            } else {
                ContentUnavailableView(
                    "No lyrics available", systemImage: "quote.bubble",
                    description: Text(
                        "Lyrics will appear here when they’re included in the song’s synced metadata."
                    ), )
            }
        }
        .foregroundStyle(.white)
        .onAppear { lyrics = PlayerLyrics(track: track) }
    }

    private func follow(_ index: Int, using proxy: ScrollViewProxy) {
        withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.65)) {
            proxy.scrollTo(index, anchor: UnitPoint(x: 0.5, y: 0.35))
        }
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
        for key in ["synchronizedLyrics", "lyrics", "unsyncedLyrics"] {
            for text in track.metadata.tags[key] ?? [] {
                let candidate = Self(text: text)
                if !candidate.lines.isEmpty {
                    self = candidate
                    return
                }
                if plain.isEmpty { plain = candidate.plain }
            }
        }
    }

    private init(text: String) {
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
                    Line(
                        seconds: max(0, minutes * 60 + seconds + fraction + offset), text: content,
                    )
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
