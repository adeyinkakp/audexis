//
//  HomePage.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct HomePage: View {
    @Environment(CloudLibrary.self) private var library
    @ScaledMetric(relativeTo: .body) private var cardWidth = 164

    private var albums: [MusicCollection] {
        MusicCollection.albums(in: library.snapshot.tracks).sorted { left, right in
            let lhs = left.tracks.map(\.added_at).max() ?? 0
            let rhs = right.tracks.map(\.added_at).max() ?? 0
            return lhs == rhs ? left.id < right.id : lhs > rhs
        }
    }

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 28) {
                CloudLibraryStatus().padding(.horizontal, 20)
                if !library.snapshot.tracks.isEmpty {
                    shortcuts
                    recentAlbums
                    if !library.snapshot.playlists.isEmpty { playlists }
                    if !library.snapshot.favorites.isEmpty { favorites }
                } else if !library.isLoading {
                    ContentUnavailableView(
                        "Your music starts here", systemImage: "music.note.house",
                        description: Text(
                            "Add music from audexis Desktop, then pull down to refresh."),
                    )
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 48)
                }
            }
            .padding(.vertical, 16)
            .frame(maxWidth: 1100)
            .frame(maxWidth: .infinity)
        }
        .refreshable {
            await library.refresh()
            await library.importDownloads()
        }
    }

    private var shortcuts: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 140), spacing: 12)], spacing: 12) {
            NavigationLink {
                FavoritesPage()
            } label: {
                shortcut(
                    "Favorites", subtitle: "\(library.snapshot.favorites.count) songs",
                    icon: "heart.fill", )
            }
            NavigationLink {
                LibraryPage(downloadedOnly: true)
            } label: {
                shortcut("Downloaded", subtitle: "Listen offline", icon: "arrow.down.circle.fill")
            }
        }
        .buttonStyle(.plain)
        .padding(.horizontal, 20)
    }

    private func shortcut(_ title: String, subtitle: String, icon: String) -> some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.title3)
                .foregroundStyle(AudexisStyle.primary)
                .frame(width: 26)
            VStack(alignment: .leading, spacing: 4) {
                Text(title).font(.subheadline.weight(.semibold))
                Text(subtitle).font(.caption).foregroundStyle(.secondary)
            }
            Spacer(minLength: 0)
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(AudexisStyle.card, in: RoundedRectangle(cornerRadius: 16))
        .contentShape(RoundedRectangle(cornerRadius: 16))
    }

    private var recentAlbums: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(alignment: .firstTextBaseline) {
                Text("Recently added").font(.title2.bold())
                Spacer(minLength: 12)
                NavigationLink {
                    LibraryPage()
                } label: {
                    Text("All music").font(.subheadline.weight(.semibold))
                }
            }
            .padding(.horizontal, 20)
            shelf {
                ForEach(albums.prefix(12)) { album in
                    NavigationLink {
                        CloudTrackCollectionPage(
                            title: album.title, subtitle: album.subtitle,
                            trackIDs: album.tracks.map(\.id), )
                    } label: {
                        CloudCollectionCard(
                            title: album.title, subtitle: album.subtitle, tracks: album.tracks,
                        )
                        .frame(width: cardWidth)
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    private var playlists: some View {
        let tracksByID = Dictionary(
            library.snapshot.tracks.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first }, )
        return VStack(alignment: .leading, spacing: 16) {
            Text("Made by you").font(.title2.bold()).padding(.horizontal, 20)
            shelf {
                ForEach(library.snapshot.playlists.prefix(12)) { playlist in
                    NavigationLink {
                        CloudPlaylistPage(playlist: playlist)
                    } label: {
                        CloudCollectionCard(
                            title: playlist.name, subtitle: "\(playlist.track_ids.count) songs",
                            tracks: playlist.track_ids.compactMap { tracksByID[$0] },
                        )
                        .frame(width: cardWidth)
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    private func shelf<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        ScrollView(.horizontal) {
            LazyHStack(alignment: .top, spacing: 16, content: content)
                .scrollTargetLayout()
        }
        .contentMargins(.horizontal, 20, for: .scrollContent)
        .scrollTargetBehavior(.viewAligned)
        .scrollIndicators(.hidden)
    }

    private var favorites: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(alignment: .firstTextBaseline) {
                Text("Songs you love").font(.title2.bold())
                Spacer(minLength: 12)
                NavigationLink {
                    FavoritesPage()
                } label: {
                    Text("See all").font(.subheadline.weight(.semibold))
                }
            }
            VStack(spacing: 4) {
                ForEach(library.snapshot.favorites.prefix(4)) { track in
                    CloudTrackRow(track: track, context: library.snapshot.favorites)
                }
            }
        }
        .padding(.horizontal, 20)
    }
}
