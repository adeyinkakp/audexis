//
//  RemoteCommands.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import MediaPlayer

nonisolated private enum MediaAction: Sendable {
  case play, pause, toggle, stop, next, previous
  case seek(Double)
  case skip(Double)
}

extension IOSMediaControls {
  func registerCommands() {
    bind(center.playCommand) { _ in .play }
    bind(center.pauseCommand) { _ in .pause }
    bind(center.togglePlayPauseCommand) { _ in .toggle }
    bind(center.stopCommand) { _ in .stop }
    bind(center.nextTrackCommand) { _ in .next }
    bind(center.previousTrackCommand) { _ in .previous }
    bind(center.changePlaybackPositionCommand) { event in
      guard let event = event as? MPChangePlaybackPositionCommandEvent,
        event.positionTime.isFinite
      else { return nil }
      return .seek(event.positionTime)
    }

  }

  private func bind(
    _ command: MPRemoteCommand,
    action: @escaping @Sendable (MPRemoteCommandEvent) -> MediaAction?
  ) {
    let target = command.addTarget { [weak self] event in
      guard let self, let action = action(event) else { return .commandFailed }
      Task { @MainActor [weak self] in
        guard let self, self.connected, let playback = self.playback, playback.track != nil else {
          return
        }
        switch action {
        case .play: playback.resume()
        case .pause: playback.pause()
        case .toggle: playback.toggle()
        case .stop:
          if let onStop = playback.onStop { onStop() } else { playback.stop() }
        case .next: playback.onNext?()
        case .previous: playback.onPrevious?()
        case .seek(let seconds): playback.seek(seconds)
        case .skip(let seconds): playback.seek(playback.position + seconds)
        }
      }
      return .success
    }
    targets.append((command, target))
  }
}
