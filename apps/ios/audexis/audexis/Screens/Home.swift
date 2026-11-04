//
//  Home.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import Combine
import SwiftUI

struct MainDashboardView: View {
    @State private var showAccount = false
    @State private var selectedTab = "Home"
    @State private var libraryPath: [CloudCollectionDestination] = []
    @State private var library = CloudLibrary()
    @Environment(\.scenePhase) private var scenePhase
    @Environment(AuthManager.self) private var authManager

    var body: some View {
        TabView(selection: $selectedTab) {
            Tab("Home", systemImage: "house.fill", value: "Home") { page("Home") { HomePage() } }
            Tab("Rewind", systemImage: "clock.arrow.circlepath", value: "Rewind") {
                page("Rewind") { RewindPage() }
            }
            Tab("Library", systemImage: "square.stack.fill", value: "Library") {
                page("Library") { LibraryPage() }
            }
            Tab("Search", systemImage: "magnifyingglass", value: "Search", role: .search) {
                page("Search") { SearchPage() }
            }
        }
        .tabViewStyle(.sidebarAdaptable)
        .environment(library)
        .environment(\.openMusicCollection, openCollection)
        .tabViewBottomAccessory {
            CloudMiniPlayer().environment(library)
                .environment(\.openMusicCollection, openCollection)
        }
        .task { await library.start() }
        .onReceive(
            NotificationCenter.default.publisher(for: CloudDownloads.changed).receive(
                on: RunLoop.main)
        ) { note in
            if let error = note.userInfo?["error"] as? String { library.error = error }
            Task { await library.importDownloads() }
        }
        .onChange(of: scenePhase) { _, phase in
            if phase == .active {
                Task {
                    await library.start()
                    await library.importDownloads()
                    await library.refresh()
                }
            }
        }
        .onDisappear {
            if authManager.state != .authenticated { library.shutdown() }
        }
        .tint(AudexisStyle.primary)
        .fullScreenCover(isPresented: $showAccount) { AccountView().environment(library) }
    }

    private func openCollection(_ destination: CloudCollectionDestination) {
        selectedTab = "Library"
        libraryPath.append(destination)
    }

    private func page<Content: View>(
        _ title: String, @ViewBuilder content: @escaping () -> Content,
    )
        -> some View
    {
        MusicTabPage(
            title: title, path: title == "Library" ? $libraryPath : nil,
            showAccount: $showAccount, content: content, )
    }
}

#Preview { MainDashboardView().environment(AuthManager()) }
