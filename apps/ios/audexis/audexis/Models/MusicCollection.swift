//
//  MusicCollection.swift
//  audexis
//
//  Created by Kp on 10/26/26.
//

import Foundation

struct MusicCollection: Identifiable {
  let id: String
  let title: String
  let subtitle: String
  let tracks: [CloudTrack]

  static func albums(in tracks: [CloudTrack]) -> [MusicCollection] {
    let groups = Dictionary(grouping: tracks) { track in
      [track.tag("albumArtist").isEmpty ? track.artist : track.tag("albumArtist"), track.album]
    }
    return groups.map { key, songs in
      MusicCollection(
        id: key.joined(separator: "\n"), title: key[1].isEmpty ? "Singles & untitled" : key[1],
        subtitle: key[0], tracks: songs.sorted { order($0) < order($1) })
    }.sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
  }

  static func artists(in tracks: [CloudTrack]) -> [MusicCollection] {
    Dictionary(grouping: tracks, by: \.artist).map { name, songs in
      MusicCollection(id: name, title: name, subtitle: "\(songs.count) songs", tracks: songs)
    }.sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
  }

  private static func order(_ track: CloudTrack) -> (Int, Int, String) {
    let disc = Int(track.tag("discnumber").split(separator: "/").first ?? "1") ?? 1
    let number = Int(track.tag("trackNumber").split(separator: "/").first ?? "0") ?? 0
    return (disc, number, track.title)
  }
}
