//
//  AccountView.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct AccountView: View {
  @Environment(AuthManager.self) private var authManager
  @Environment(CloudLibrary.self) private var library
  @Environment(\.dismiss) private var dismiss
  @State private var showServerSettings = false

  private var custom: Bool { authManager.connection?.kind == .custom }

  var body: some View {
    NavigationStack {
      List {
        Section {
          VStack(alignment: .leading, spacing: 14) {
            Image(systemName: custom ? "server.rack" : "cloud.fill")
              .font(.largeTitle).foregroundStyle(AudexisStyle.primary)
              .frame(width: 68, height: 68).background(
                AudexisStyle.primary_soft, in: RoundedRectangle(cornerRadius: 18), )
            Text(custom ? "Your server. Your music." : "Your Audexis account").font(
              .title2.bold())
            Text(
              custom
                ? "Connected to your own Audexis server."
                : "Your library travels with you."
            )
            .font(.subheadline).foregroundStyle(.secondary)
          }.padding(.vertical, 12).listRowBackground(Color.clear)
        }
        Section("Connection") {
          Label(
            custom ? "Custom server" : "Audexis Cloud",
            systemImage: custom ? "server.rack" : "cloud", )
          if let connection = authManager.connection {
            Text(connection.url).font(.subheadline).foregroundStyle(.secondary)
              .textSelection(.enabled)
          }
          if custom { Button("Edit server details") { showServerSettings = true } }
        }
        Section("On this device") {
          LabeledContent("Library", value: "\(library.snapshot.tracks.count) songs")
          LabeledContent("Downloaded", value: "\(library.snapshot.downloads.count) songs")
          if !library.downloads.isEmpty {
            LabeledContent("Downloading", value: "\(library.downloads.count) songs")
          }
        }
        Section {
          Button("Sign out", role: .destructive) {
            library.shutdown()
            Task { await authManager.remoteLogout() }
          }
        }
      }
      .scrollContentBackground(.hidden).background(AudexisStyle.background)
      .navigationTitle("Account").navigationBarTitleDisplayMode(.inline)
      .toolbar {
        ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } }
      }
      .sheet(isPresented: $showServerSettings) { CustomServerForm() }
    }
    .presentationDetents([.large])
    .tint(AudexisStyle.primary)
  }
}
