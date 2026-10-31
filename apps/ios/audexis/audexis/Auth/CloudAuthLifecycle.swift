//
//  CloudAuthLifecycle.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import AuthenticationServices
import Foundation

extension CloudAuthService {
  func beginAttempt() -> UUID {
    let attempt = UUID()
    attemptID = attempt
    isLoading = true
    armTimeout(for: attempt, seconds: 180)
    return attempt
  }

  func cancelSignIn(message: String? = nil) {
    attemptID = nil
    completionTask?.cancel()
    timeoutTask?.cancel()
    authenticationSession?.cancel()
    completionTask = nil
    timeoutTask = nil
    authenticationSession = nil
    presentationContext = nil
    isLoading = false
    errorMessage = message
  }

  func armTimeout(for attempt: UUID, seconds: Double) {
    timeoutTask?.cancel()
    timeoutTask = Task { @MainActor [weak self] in
      do { try await Task.sleep(for: .seconds(seconds)) } catch { return }
      guard let self, self.attemptID == attempt else { return }
      self.cancelSignIn(message: "Sign-in timed out. Please try again or use your own server.")
    }
  }

  func finishAttempt(_ attempt: UUID) {
    guard attemptID == attempt else { return }
    attemptID = nil
    timeoutTask?.cancel()
    timeoutTask = nil
    completionTask = nil
    authenticationSession = nil
    presentationContext = nil
    isLoading = false
  }
}
