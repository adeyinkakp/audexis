//
//  CloudSignInError.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import Foundation

enum CloudSignInError: LocalizedError {
  case invalidConfiguration, invalidResponse, expired, subscriptionRequired, serverUnavailable,
    invalidSession

  var errorDescription: String? {
    switch self {
    case .invalidConfiguration: "The sign-in server address is invalid."
    case .invalidResponse: "The sign-in server returned an invalid response. Please try again."
    case .expired: "Your sign-in expired or could not be completed. Please try again."
    case .subscriptionRequired: "An active Audexis Cloud subscription is required."
    case .serverUnavailable:
      "The sign-in server is temporarily unavailable. Please try again later."
    case .invalidSession: "Could not verify your cloud session. Please sign in again."
    }
  }
}
