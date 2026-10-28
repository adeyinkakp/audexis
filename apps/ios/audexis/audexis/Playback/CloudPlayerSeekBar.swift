//
//  CloudPlayerSeekBar.swift
//  audexis
//
//  Created by Kp on 10/26/26.
//

import Foundation
import SwiftUI

struct CloudPlayerSeekBar: View {
  let position: Double
  let duration: Double
  let onSeek: (Double) -> Void
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @GestureState private var isDragging = false
  @State private var draft: Double = 0

  private var length: Double { duration.isFinite ? max(0, duration) : 0 }
  private var progress: Double {
    let value = isDragging ? draft : position
    return value.isFinite ? min(max(0, value), length) : 0
  }

  var body: some View {
    VStack(spacing: 0) {
      GeometryReader { geometry in
        ZStack(alignment: .leading) {
          Capsule().fill(AudexisStyle.control_fill)
          Capsule().fill(AudexisStyle.foreground.opacity(isDragging ? 1 : 0.65))
            .frame(width: geometry.size.width * (length > 0 ? progress / length : 0))
        }
        .frame(height: isDragging ? 10 : 5)
        .frame(maxHeight: .infinity)
        .contentShape(Rectangle())
        .gesture(
          DragGesture(minimumDistance: 0)
            .updating($isDragging) { _, state, _ in state = true }
            .onChanged { value in draft = target(value.location.x, width: geometry.size.width) }
            .onEnded { value in
              guard length > 0 else { return }
              onSeek(target(value.location.x, width: geometry.size.width))
            }
        )
        .allowsHitTesting(length > 0)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.15), value: isDragging)
      }
      .frame(height: 44)
      HStack {
        Text(time(progress))
        Spacer()
        Text("−" + time(max(0, length - progress)))
      }
      .font(.caption2.weight(.medium).monospacedDigit())
      .foregroundStyle(isDragging ? AudexisStyle.foreground : AudexisStyle.content_secondary)
    }
    .accessibilityElement(children: .ignore)
    .accessibilityLabel("Playback position")
    .accessibilityValue("\(time(progress)) of \(time(length))")
    .accessibilityAdjustableAction { direction in
      guard length > 0 else { return }
      switch direction {
      case .increment: onSeek(min(length, progress + 5))
      case .decrement: onSeek(max(0, progress - 5))
      @unknown default: break
      }
    }
  }

  private func target(_ x: CGFloat, width: CGFloat) -> Double {
    guard width > 0 else { return 0 }
    return min(max(0, Double(x / width)), 1) * length
  }

  private func time(_ seconds: Double) -> String {
    let value = Int(seconds)
    return String(format: "%d:%02d", value / 60, value % 60)
  }
}
