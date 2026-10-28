//
//  Artwork.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import MediaPlayer
import UIKit

extension IOSMediaControls {
  func setArtwork(trackID: String, image: UIImage) {
    guard connected, playback?.track?.id == trackID else { return }
    artworkID = trackID
    artwork = MPMediaItemArtwork(boundsSize: image.size) { _ in image }
    updateNowPlaying()
  }
}
