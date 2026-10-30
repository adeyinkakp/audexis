//
//  FavoritesPage.swift
//  audexis
//
//  Created by Kp on 10/29/26.
//

import SwiftUI

struct FavoritesPage: View {
  @Environment(CloudLibrary.self) private var library
  var downloadedOnly = false

  private var tracks: [CloudTrack] {
    library.snapshot.favorites.filter {
      !downloadedOnly || library.snapshot.downloads[$0.id] != nil
    }
  }

  var body: some View {
    CloudTrackCollectionPage(
      title: "Favorites", subtitle: downloadedOnly ? "Downloaded favorites" : "Songs you love",
      trackIDs: tracks.map(\.id),
      emptyTitle: downloadedOnly ? "No downloaded favorites" : "No favorites yet",
      emptyDescription: downloadedOnly
        ? "Download your favorite songs to listen offline."
        : "You haven't added any songs to ya favorites!", )
  }
}

struct FavoritesPlaylistLink: View {
  @Environment(CloudLibrary.self) private var library
  let downloadedOnly: Bool
  let chevronHidden: Bool

  init(_chevHidden: Bool, _downloadOnly: Bool) {
    self.chevronHidden = _chevHidden
    self.downloadedOnly = _downloadOnly
  }

  private var count: Int {
    library.snapshot.favorites.filter {
      !downloadedOnly || library.snapshot.downloads[$0.id] != nil
    }.count
  }

  var body: some View {
    NavigationLink {
      FavoritesPage(downloadedOnly: downloadedOnly)
    } label: {
      HStack(spacing: 14) {
        Image(systemName: "heart.fill").font(.title2)
          .foregroundStyle(AudexisStyle.primary)
          .frame(width: 52, height: 52)
          .background(AudexisStyle.primary_soft, in: RoundedRectangle(cornerRadius: 12))
        VStack(alignment: .leading, spacing: 4) {
          Text("Favorites").font(.headline)
          Text("\(count) songs").font(.caption).foregroundStyle(.secondary)
        }
        Spacer(minLength: 0)
        if !self.chevronHidden {
          Image(systemName: "chevron.right").font(.caption).foregroundStyle(.secondary)
        }

      }
      .padding(14).background(AudexisStyle.card, in: RoundedRectangle(cornerRadius: 16))
    }
    .buttonStyle(.plain)
  }
}
