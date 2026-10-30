//
//  Rewind.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct RewindPage: View {
  @Environment(CloudLibrary.self) private var library
  @State private var year = Calendar.current.component(.year, from: Date())
  @State private var month = 0
  @State private var rewind: CloudRewind?
  @State private var isLoading = false
  @State private var error: String?
  @State private var requestID = UUID()
  @State private var availableYears: [Int] = []

  private var years: [Int] {
    Array(
      Set(availableYears + [year, Calendar.current.component(.year, from: Date())])
    )
    .sorted(by: >)
  }
  private var period: String {
    month == 0 ? String(year) : "\(Calendar.current.monthSymbols[month - 1]) \(year)"
  }
  private var tracksByID: [String: CloudTrack] {
    Dictionary(
      library.snapshot.tracks.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first }, )
  }
  private var topTracks: [CloudTrack] {
    (rewind?.summary.songs ?? []).compactMap { track(for: $0) }
  }

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 28) {
        HStack {
          Picker("Year", selection: $year) {
            ForEach(years, id: \.self) { Text(String($0)).tag($0) }
          }
          Picker("Month", selection: $month) {
            Text("Full year").tag(0)
            ForEach(1...12, id: \.self) { Text(Calendar.current.monthSymbols[$0 - 1]).tag($0) }
          }
          Spacer(minLength: 0)
          if isLoading { ProgressView().accessibilityLabel("Loading Rewind") }
        }
        .pickerStyle(.menu)

        if let error {
          InlineError(message: error)
          Button("Try again") { Task { await load() } }
        }
        if let rewind {
          summary(rewind.summary)
          if rewind.summary.total_us > 0 {
            songs(rewind.summary.songs)
            ranking("Top artists", entries: rewind.summary.artists, albums: false)
            ranking("Top albums", entries: rewind.summary.albums, albums: true)
          } else {
            ContentUnavailableView(
              "Your Rewind starts here", systemImage: "headphones",
              description: Text("Listen to music in Audexis to see your favorites for \(period)."),
            )
          }
        } else if library.isLoading || isLoading {
          ProgressView("Loading your listening history…")
            .frame(maxWidth: .infinity).padding(.vertical, 60)
        }
      }
      .padding(20).frame(maxWidth: 1100).frame(maxWidth: .infinity)
    }
    .task(id: "\(year)-\(month)-\(library.isLoading)") { await load() }
    .refreshable { await load() }
  }

  private func summary(_ summary: CloudRewind.Summary) -> some View {
    VStack(alignment: .leading, spacing: 12) {
      Label("Your \(period) in music", systemImage: "clock.arrow.circlepath")
        .font(.headline).foregroundStyle(AudexisStyle.primary)
      Text(listeningTime(summary.total_us)).font(.largeTitle.bold())
      Text("spent listening").font(.title3).foregroundStyle(.secondary)
      Text("Listening history from this device. Pull down to refresh.")
        .font(.caption).foregroundStyle(.secondary)
    }
    .frame(maxWidth: .infinity, alignment: .leading).padding(24)
    .background(AudexisStyle.primary_washed, in: RoundedRectangle(cornerRadius: 24))
  }

  @ViewBuilder
  private func songs(_ entries: [CloudRewind.Entry]) -> some View {
    if !topTracks.isEmpty {
      VStack(alignment: .leading, spacing: 16) {
        Text("Top songs").font(.title2.bold())
        CloudCollectionActions(tracks: topTracks)
        LazyVStack(spacing: 14) {
          ForEach(Array(entries.enumerated()), id: \.offset) { index, entry in
            if let track = track(for: entry) {
              HStack(alignment: .top, spacing: 12) {
                rank(index)
                VStack(alignment: .leading, spacing: 2) {
                  CloudTrackRow(track: track, context: topTracks)
                  Text(listeningTime(entry.listened_us)).font(.caption).foregroundStyle(.secondary)
                }
              }
            }
          }
        }
      }
    }
  }

  @ViewBuilder
  private func ranking(_ title: String, entries: [CloudRewind.Entry], albums: Bool) -> some View {
    if !entries.isEmpty {
      VStack(alignment: .leading, spacing: 16) {
        Text(title).font(.title2.bold())
        LazyVStack(spacing: 12) {
          ForEach(Array(entries.enumerated()), id: \.offset) { index, entry in
            NavigationLink {
              CloudTrackCollectionPage(
                title: entry.name ?? (albums ? "Unknown album" : "Unknown artist"),
                subtitle: albums ? entry.artist ?? "" : "Artist",
                trackIDs: collectionTracks(entry, albums: albums).map(\.id), )
            } label: {
              HStack(spacing: 12) {
                rank(index)
                if let track = track(for: entry) { CloudArtwork(track: track, size: 52) }
                VStack(alignment: .leading, spacing: 4) {
                  Text(entry.name ?? "Unknown").font(.headline).lineLimit(1)
                  if albums, let artist = entry.artist, !artist.isEmpty {
                    Text(artist).font(.subheadline).foregroundStyle(.secondary).lineLimit(1)
                  }
                  Text(listeningTime(entry.listened_us)).font(.caption).foregroundStyle(.secondary)
                }
                Spacer(minLength: 0)
                Image(systemName: "chevron.right").font(.caption).foregroundStyle(.secondary)
              }
              .padding(14).background(AudexisStyle.card, in: RoundedRectangle(cornerRadius: 16))
            }
            .buttonStyle(.plain)
          }
        }
      }
    }
  }

  private func rank(_ index: Int) -> some View {
    Text(String(index + 1)).font(.headline.monospacedDigit()).foregroundStyle(.secondary)
      .frame(minWidth: 24).padding(.top, 12).accessibilityLabel("Rank \(index + 1)")
  }

  private func track(for entry: CloudRewind.Entry) -> CloudTrack? {
    guard let id = rewind?.track_ids[String(entry.file_id)] else { return nil }
    return tracksByID[id]
  }

  private func collectionTracks(_ entry: CloudRewind.Entry, albums: Bool) -> [CloudTrack] {
    func matches(_ a: String, _ b: String) -> Bool {
      a.trimmingCharacters(in: .whitespacesAndNewlines).localizedCaseInsensitiveCompare(b)
        == .orderedSame
    }
    let tracks = library.snapshot.tracks.filter { track in
      if albums {
        return matches(track.album, entry.name ?? "")
          && matches(track.tag("artist"), entry.artist ?? "")
      }
      return (track.metadata.tags["artist"] ?? []).contains { matches($0, entry.name ?? "") }
    }
    return albums ? MusicCollection.albums(in: tracks).flatMap(\.tracks) : tracks
  }

  private func listeningTime(_ microseconds: Int64) -> String {
    let seconds = max(0, microseconds / 1_000_000)
    if seconds < 60 { return "\(seconds) sec" }
    let minutes = seconds / 60
    if minutes < 60 { return "\(minutes) min" }
    return "\(minutes / 60) hr \(minutes % 60) min"
  }

  private func load() async {
    guard !library.isLoading else { return }
    let id = UUID()
    requestID = id
    isLoading = true
    error = nil
    rewind = nil
    defer { if requestID == id { isLoading = false } }
    do {
      let result = try await library.rewind(year: year, month: month == 0 ? nil : month)
      guard !Task.isCancelled, requestID == id else { return }
      rewind = result
      availableYears = result.summary.years
    } catch {
      guard !Task.isCancelled, requestID == id else { return }
      self.error = error.localizedDescription
    }
  }
}
