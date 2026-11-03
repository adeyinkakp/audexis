//
//  CloudTrackDetails.swift
//  audexis
//
//  Created by Kp on 10/22/26.
//

import SwiftUI
import MarqueeText

struct CloudTrackDetails: View {
  let track: CloudTrack
  var titleFont: Font = .body
  var subtitle: String? = nil
  var highlighted = false

  var body: some View {
    VStack(alignment: .leading, spacing: 1) {
        
        MarqueeText(
            text: track.title,
            font: UIFont.preferredFont(forTextStyle: .body),
            leftFade: 16,
            rightFade: 16,
            startDelay: 5
        
            ).font(titleFont)
            .foregroundStyle(highlighted ? AudexisStyle.primary : AudexisStyle.foreground)
            .lineLimit(1)
   
        
      Text(subtitle ?? track.artist)
        .font(.caption)
        .foregroundStyle(.secondary)
        .lineLimit(1)
    }
  }
}
