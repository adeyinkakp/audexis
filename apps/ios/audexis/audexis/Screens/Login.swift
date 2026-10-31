//
//  Login.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct LoginView: View {
  @Environment(AuthManager.self) private var authManager
  @State private var authService = CloudAuthService()
  @State private var showCustomServer = false

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 32) {
        VStack(alignment: .leading, spacing: 20) {
          Image("SplashIcon").resizable().scaledToFit().frame(width: 88, height: 88)
            .accessibilityHidden(true)
          Text("All your music.\nRight here.").font(.largeTitle.bold())
          Text(
            "Bring your library with Audexis Cloud or connect your own server. Stream or Download tracks"
          )
          .font(.body).foregroundStyle(.secondary)
        }
        .padding(.top, 32)
        VStack(spacing: 14) {
          if let error = authService.errorMessage { InlineError(message: error) }
          Button {
            authService.signIn(authManager: authManager)
          } label: {
            HStack(spacing: 12) {
              if authService.isLoading { ProgressView().tint(.white) }
              Text(authService.isLoading ? "Connecting…" : "Continue with Audexis Cloud")
              if !authService.isLoading { Image(systemName: "arrow.right") }
            }
            .font(.subheadline.bold()).frame(maxWidth: .infinity).padding(.vertical, 12)
          }
          .buttonStyle(.borderedProminent).buttonBorderShape(.roundedRectangle(radius: 14))
          .disabled(authService.isLoading)
          if authService.isLoading {
            Button("Cancel sign-in") { authService.cancelSignIn() }
              .font(.subheadline.weight(.semibold))
          }
          Button {
            authService.cancelSignIn()
            showCustomServer = true
          } label: {
            Label("Connect your own server", systemImage: "server.rack")
              .font(.subheadline.weight(.semibold)).frame(maxWidth: .infinity).padding(
                .vertical, 12, )
          }
          .buttonStyle(.bordered).buttonBorderShape(.roundedRectangle(radius: 14))
          Text(
            "Cloud sign-in requires an active subscription.\nAlready hosting Audexis? Use your server URL and API key."
          )
          .font(.caption).foregroundStyle(AudexisStyle.muted_foreground).multilineTextAlignment(
            .center)
        }
        .padding(20).background(AudexisStyle.card, in: RoundedRectangle(cornerRadius: 24))
      }
      .padding(26).frame(maxWidth: 500).frame(maxWidth: .infinity)
    }
    .background(AudexisStyle.background).tint(AudexisStyle.primary)
    .onDisappear { authService.cancelSignIn() }
    .sheet(isPresented: $showCustomServer) { CustomServerForm() }
  }
}

#Preview { LoginView().environment(AuthManager()) }
