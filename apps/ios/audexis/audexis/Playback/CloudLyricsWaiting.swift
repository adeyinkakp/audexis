//
//  CloudLyricsWaiting.swift
//  audexis
//
//  Created by Kp on 10/26/26.
//

import Foundation
import SwiftUI

struct CloudLyricsWaiting: View {
  let position: Double
  let start: Double
  let end: Double?
  @Environment(\.accessibilityReduceMotion) private var reduceMotion

  private var progress: Double {
    let elapsed = max(0, position - start)
    if let end, end > start { return min(1, elapsed / (end - start)) }
    return elapsed.truncatingRemainder(dividingBy: 2.4) / 2.4
  }

  var body: some View {
    HStack(spacing: 6) {
      ForEach(0..<3) { index in
        let fill = min(1, max(0, progress * 3 - Double(index)))
        Circle()
          .fill(AudexisStyle.primary)
          .frame(width: 8, height: 8)
          .opacity(0.3 + fill * 0.7)
          .scaleEffect(reduceMotion ? 1 : 1 + sin(fill * .pi) * 0.3)
      }
    }
    .frame(height: 43)
    .frame(maxWidth: .infinity, alignment: .leading)
    .animation(reduceMotion ? nil : .linear(duration: 0.15), value: progress)
    .accessibilityElement(children: .ignore)
    .accessibilityLabel("Instrumental break. Lyrics resume shortly.")
  }
}
