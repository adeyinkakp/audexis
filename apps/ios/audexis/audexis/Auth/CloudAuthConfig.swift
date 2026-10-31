//
//  CloudAuthConfig.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import Foundation

enum CloudAuthConfiguration {
  static var origin: String {
    Bundle.main.object(forInfoDictionaryKey: "AudexisAuthOrigin") as? String
      ?? "http://localhost:4321"
  }
}
