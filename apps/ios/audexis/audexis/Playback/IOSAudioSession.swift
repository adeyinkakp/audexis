//
//  IOSAudioSession.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import AVFAudio

actor IOSAudioSession {
  static let shared = IOSAudioSession()
  private var configured = false
  private var revision: UInt64 = 0

  func activate(revision requested: UInt64) throws {
    guard requested >= revision else { return }
    revision = requested
    let session = AVAudioSession.sharedInstance()
    if !configured {
      try session.setCategory(.playback, mode: .default)
      configured = true
    }
    try session.setActive(true)
  }

  func deactivate(revision requested: UInt64) {
    guard requested >= revision else { return }
    revision = requested
    try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
  }
}
