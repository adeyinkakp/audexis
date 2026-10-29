//
//  MobileCore.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import Foundation

private nonisolated struct MobileResponse<Body: Decodable>: Decodable {
  let ok: Body?
  let error: String?
}

actor MobileCore {
  private final class Handle: @unchecked Sendable {
    let pointer = audexis_mobile_create()!

    let queue = DispatchQueue(label: "com.kp.audexis.mobile-core", qos: .default)

    deinit {
      let address = UInt(bitPattern: pointer)
      queue.async { audexis_mobile_destroy(OpaquePointer(bitPattern: address)) }
    }
  }
  private let handle = Handle()

  func openLibrary(at directory: URL) async throws -> CloudSnapshot {
    try await call(.init(op: "open", directory: directory.path))
  }

  func librarySnapshot() async throws -> CloudSnapshot {
    try await call(.init(op: "snapshot"))
  }

  func call<T: Decodable & Sendable>(_ command: MobileCommand, as type: T.Type = T.self)
    async throws
    -> T
  {
    let bytes = try JSONEncoder().encode(command)
    let request = String(decoding: bytes, as: UTF8.self)
    let handle = self.handle
    let json: String = try await withCheckedThrowingContinuation { continuation in
      handle.queue.async {
        let result = request.withCString { audexis_mobile_call(handle.pointer, $0) }
        guard let result else {
          continuation.resume(
            throwing: CloudError.message("Could not open the music library."))
          return
        }
        let response = String(cString: result)
        audexis_mobile_string_free(result)
        continuation.resume(returning: response)
      }
    }
    let data = Data(json.utf8)
    let response = try JSONDecoder().decode(MobileResponse<T>.self, from: data)
    if let error = response.error { throw CloudError.message(error) }
    guard let value = response.ok else { throw CloudError.message("Invalid library response.") }
    return value
  }
}
