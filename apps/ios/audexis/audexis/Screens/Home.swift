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
  @State private var library = CloudLibrary()
  @Environment(\.scenePhase) private var scenePhase
  @Environment(AuthManager.self) private var authManager

  var body: some View {
    TabView {
      Tab("Home", systemImage: "house.fill") { page("Home") { HomePage() } }
      Tab("Rewind", systemImage: "clock.arrow.circlepath") { page("Rewind") { RewindPage() } }
      Tab("Library", systemImage: "square.stack.fill") { page("Library") { LibraryPage() } }
      Tab("Search", systemImage: "magnifyingglass", role: .search) {
        page("Search") { SearchPage() }
      }
    }
    .tabViewStyle(.sidebarAdaptable)
    .environment(library)
    .tabViewBottomAccessory { CloudMiniPlayer().environment(library) }
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
    .sheet(isPresented: $showAccount) { AccountView().environment(library) }
  }

  private func page<Content: View>(_ title: String, @ViewBuilder content: @escaping () -> Content)
    -> some View
  {
    MusicTabPage(title: title, showAccount: $showAccount, content: content)
  }
}

#Preview { MainDashboardView().environment(AuthManager()) }
