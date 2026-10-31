//
//  audexisApp.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

@main
struct audexisApp: App {
  @UIApplicationDelegateAdaptor(AudexisAppDelegate.self) private var appDelegate
  @State private var authManager = AuthManager()

  var body: some Scene {
    WindowGroup {
      ZStack {
        AudexisStyle.background.ignoresSafeArea()
        switch authManager.state {
        case .checking:
          LoadingSplashScreen()
        case .unauthenticated:
          LoginView()
        case .authenticated:
          MainDashboardView()
        }
      }
      .environment(authManager)
      .task {
        if authManager.state == .checking {
          await authManager.checkExistingSession()
        }
      }
    }
  }
}
