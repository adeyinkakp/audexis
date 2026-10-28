//
//  CloudDownloadIndicator.swift
//  audexis
//
//  Created by Kp on 10/22/26.
//

import SwiftUI

struct CloudDownloadIndicator: View {
  @Environment(CloudLibrary.self) private var library
  let track: CloudTrack

  var body: some View {
    if library.snapshot.downloads[track.id] != nil {
      Image(systemName: "arrow.down.circle.fill")
        .foregroundStyle(AudexisStyle.primary)
        .accessibilityLabel("Downloaded")
    } else if let progress = library.downloads[track.id] {
      ProgressView(value: progress)
        .progressViewStyle(.circular)
        .frame(width: 20)
        .accessibilityLabel("Downloading")
        .accessibilityValue(Text(progress, format: .percent.precision(.fractionLength(0))))
    }
  }
}
