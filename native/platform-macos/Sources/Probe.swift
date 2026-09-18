import AppKit
import ScreenCaptureKit
import VideoToolbox
import CoreImage
import ApplicationServices

// Phase 0 helper: no service, no persistence, no hidden capture. A visible window
// and an explicit local Start button precede every real-screen experiment.
let args = CommandLine.arguments
func option(_ name: String, _ fallback: String) -> String {
    guard let i = args.firstIndex(of: name), i + 1 < args.count else { return fallback }
    return args[i + 1]
}
func json(_ object: [String: Any]) -> Data {
    (try? JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])) ?? Data()
}
func diagnostic(_ object: [String: Any]) {
    FileHandle.standardError.write(json(object) + Data([10]))
}
let screenAllowed = CGPreflightScreenCaptureAccess()
let inputAllowed = AXIsProcessTrusted()
if args.contains("--request-permission") {
    let permission = option("--request-permission", "")
    let app = NSApplication.shared
    app.setActivationPolicy(.accessory)
    let pane: String
    switch permission {
    case "screen":
        if !screenAllowed { _ = CGRequestScreenCaptureAccess() }
        pane = "Privacy_ScreenCapture"
    case "input":
        let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        _ = AXIsProcessTrustedWithOptions(options)
        pane = "Privacy_Accessibility"
    default:
        exit(2)
    }
    let opened = NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?\(pane)")!)
    FileHandle.standardOutput.write(json(["settingsOpened": opened]) + Data([10]))
    exit(opened ? 0 : 1)
}
if args.contains("--status") {
    let status: [String: Any] = ["platform": "macos", "screenRecording": screenAllowed,
        "accessibility": inputAllowed, "capture": "ScreenCaptureKit", "codec": "H264/VideoToolbox",
        "secureDesktop": false, "remoteControlVerified": false,
        "os": ProcessInfo.processInfo.operatingSystemVersionString]
    FileHandle.standardOutput.write(json(status) + Data([10]))
    exit(0)
}

enum ProbeError: Error { case failure(String) }
func check(_ status: OSStatus, _ label: String) throws {
    if status != noErr { throw ProbeError.failure("\(label): OSStatus \(status)") }
}
func be32(_ n: Int) -> Data { var v = UInt32(n).bigEndian; return Data(bytes: &v, count: 4) }
func readExact(_ n: Int) throws -> Data {
    var data = Data()
    while data.count < n {
        guard let part = try FileHandle.standardInput.read(upToCount: n - data.count), !part.isEmpty
        else { throw ProbeError.failure("pipe_closed") }
        data.append(part)
    }
    return data
}
func number(_ d: Data, _ p: Int) -> Int {
    (0..<4).reduce(0) { ($0 << 8) | Int(d[p + $1]) }
}

// UI is main-thread only; lifecycle/counters and codec handles use separate locks.
final class Probe: NSObject, SCStreamOutput, SCStreamDelegate, NSWindowDelegate, @unchecked Sendable {
    var window: NSWindow!
    let status = NSTextField(labelWithString: "Yerel deney • Ekran paylaşımı başlamadı")
    let preview = NSView()
    let start = NSButton(title: "Ekran testini başlat", target: nil, action: nil)
    let stop = NSButton(title: "Durdur", target: nil, action: nil)
    var stream: SCStream?
    var encoder: VTCompressionSession?
    var decoder: VTDecompressionSession?
    var format: CMVideoFormatDescription?
    var sps = Data(), pps = Data()
    let captureQueue = DispatchQueue(label: "dx.capture", qos: .userInteractive)
    let decodeQueue = DispatchQueue(label: "dx.decode", qos: .userInteractive)
    let rendering = DispatchSemaphore(value: 1)
    let statsLock = NSLock()
    let lifecycleLock = NSLock()
    let encoderLock = NSRecursiveLock()
    let decoderLock = NSRecursiveLock()
    let context = CIContext(options: [.cacheIntermediates: false])
    var encoded = 0, decoded = 0, encodedBytes = 0, renderDrops = 0
    var started = Date(), finishing = false
    private var isRunning = false
    var running: Bool {
        get { lifecycleLock.withLock { isRunning } }
        set { lifecycleLock.withLock { isRunning = newValue } }
    }
    let receiveOnly = args.contains("--receive-only")
    var synthetic = args.contains("--codec-test")
    let bridge = args.contains("--bridge")
    var hardware = false
    let remote = args.contains("--remote-session")
    let seconds = min(args.contains("--remote-session") ? 1800 : 60, max(2, Int(option("--seconds", "15")) ?? 15))
    var width = 0, height = 0
    var inputMouseClicks = 0
    @objc func inputMouseClicked() { inputMouseClicks += 1 }

    func show() {
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 900, height: 620),
            styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        window.title = remote ? "dengeX Remote — Ekran paylaşımı açık · Durdur ile sonlandırın" : "dengeX Remote — Native doğrulama (geliştirme)"
        window.center(); window.delegate = self
        let root = NSStackView(); root.orientation = .vertical; root.spacing = 16
        root.edgeInsets = NSEdgeInsets(top: 20, left: 24, bottom: 20, right: 24)
        let title = NSTextField(labelWithString: synthetic ? "Sentetik codec testi" : "Gerçek ekran · H.264 · native görüntü")
        title.font = .systemFont(ofSize: 22, weight: .semibold)
        let subtitle = NSTextField(wrappingLabelWithString: remote
            ? "Ekranınız onayladığınız davetli cihaza şifreli WebRTC bağlantısıyla aktarılıyor. Yalnızca izleme; klavye/fare erişimi yok. Durdur veya pencereyi kapatma paylaşımı bitirir."
            : bridge
            ? "Bu cihazdaki iki WebRTC uç noktası arasında DTLS/SRTP deneyi. İkinci bilgisayarla bağlantı kurulmaz. Ekran içeriği dosyaya kaydedilmez."
            : "Bu cihazda ScreenCaptureKit → VideoToolbox encode/decode testi. Ekran içeriği ağ üzerinden gönderilmez veya kaydedilmez.")
        preview.wantsLayer = true; preview.layer?.backgroundColor = NSColor.black.cgColor
        preview.layer?.contentsGravity = .resizeAspect
        preview.heightAnchor.constraint(greaterThanOrEqualToConstant: 340).isActive = true
        start.target = self; start.action = #selector(begin)
        stop.target = self; stop.action = #selector(end); stop.isEnabled = false
        let buttons = NSStackView(views: [start, stop]); buttons.spacing = 12
        for v in [title, subtitle, preview, status, buttons] { root.addArrangedSubview(v) }
        root.alignment = .leading
        preview.widthAnchor.constraint(equalTo: root.widthAnchor, constant: -48).isActive = true
        window.contentView = root
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        if receiveOnly {
            running = true; started = Date(); start.isEnabled = false; stop.isEnabled = true
            status.stringValue = "Davetli alıcı • Şifreli hattan gelen gerçek ekran bekleniyor"
            readReturnedVideo()
            DispatchQueue.main.asyncAfter(deadline: .now() + .seconds(seconds)) { self.end() }
        } else if synthetic || args.contains("--start-local-test") { begin() }
        if args.contains("--input-test") { inputSelfTest() }
        if remote {
            // Parent crash/quit closes the pipe. No orphan background sharing.
            DispatchQueue.global().async {
                while let data = try? FileHandle.standardInput.read(upToCount: 1), !data.isEmpty {}
                DispatchQueue.main.async { self.end() }
            }
        }
    }

    func inputSelfTest() {
        guard AXIsProcessTrusted() else {
            fail("accessibility_permission_required"); return
        }
        start.isEnabled = false
        status.stringValue = "Giriş testi: yalnızca bu test penceresine Türkçe metin gönderiliyor"
        let field = NSTextField(string: "")
        field.frame = NSRect(x: 40, y: 120, width: 700, height: 42)
        preview.addSubview(field)
        let mouseTarget = NSButton(title: "Fare testi hedefi", target: self, action: #selector(inputMouseClicked))
        mouseTarget.frame = NSRect(x: 40, y: 55, width: 220, height: 40)
        preview.addSubview(mouseTarget)
        window.makeFirstResponder(field)
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            let expected = "dengeX ğüşİöç"
            let units = Array(expected.utf16)
            guard let down = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true),
                  let up = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false) else {
                self.fail("input_event_creation"); return
            }
            down.keyboardSetUnicodeString(stringLength: units.count, unicodeString: units)
            up.keyboardSetUnicodeString(stringLength: units.count, unicodeString: units)
            down.postToPid(getpid()); up.postToPid(getpid())
            DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
                let received = field.currentEditor()?.string ?? field.stringValue
                let rect = mouseTarget.convert(mouseTarget.bounds, to: nil)
                let location = NSPoint(x: rect.midX, y: rect.midY)
                for (index, type) in [NSEvent.EventType.mouseMoved, .leftMouseDown, .leftMouseUp].enumerated() {
                    if let event = NSEvent.mouseEvent(with: type, location: location, modifierFlags: [],
                        timestamp: ProcessInfo.processInfo.systemUptime,
                        windowNumber: self.window.windowNumber, context: nil, eventNumber: index + 1,
                        clickCount: 1, pressure: type == .leftMouseDown ? 1 : 0)?.cgEvent {
                        event.postToPid(getpid())
                    }
                }
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.7) {
                let result: [String: Any] = ["schemaVersion": 1, "experiment": "native_input_own_window",
                    "result": received == expected && self.inputMouseClicks == 1 ? "passed" : "failed",
                    "unicodeTextVerified": received == expected,
                    "mouseClickVerified": self.inputMouseClicks == 1,
                    "receivedCodeUnits": received.utf16.count,
                    "remoteInputTest": false, "physicalKeyLayoutTest": false,
                    "twoDeviceTest": false, "target": "own_test_process_only"]
                diagnostic(result)
                let path = option("--report", "")
                if !path.isEmpty { try? json(result).write(to: URL(fileURLWithPath: path), options: .atomic) }
                NSApp.terminate(nil)
                }
            }
        }
    }

    @objc func begin() {
        guard !running, !finishing else { return }
        if !synthetic && !CGPreflightScreenCaptureAccess() {
            // Permission requests are a separate UI action. Never leave the media pipe
            // hanging while waiting for System Settings or a restart.
            endWithResult("screen_recording_permission_required")
            return
        }
        running = true; started = Date(); start.isEnabled = false; stop.isEnabled = true
        status.stringValue = "● Test sürüyor — Durdur düğmesi anında sonlandırır"
        Task {
            do {
                if synthetic { try setupEncoder(width: 320, height: 180); syntheticFrames() }
                else { try await capture() }
                if bridge && !remote { readReturnedVideo() }
                DispatchQueue.main.asyncAfter(deadline: .now() + .seconds(seconds)) { self.end() }
            } catch { fail(String(describing: error)) }
        }
    }

    func setupEncoder(width: Int, height: Int) throws {
        encoderLock.lock(); defer { encoderLock.unlock() }
        self.width = width; self.height = height
        let spec = [kVTVideoEncoderSpecification_EnableHardwareAcceleratedVideoEncoder: true] as CFDictionary
        try check(VTCompressionSessionCreate(allocator: kCFAllocatorDefault,
            width: Int32(width), height: Int32(height), codecType: kCMVideoCodecType_H264,
            encoderSpecification: spec, imageBufferAttributes: nil, compressedDataAllocator: nil,
            outputCallback: { ref, _, status, _, sample in
                guard let ref, let sample, status == noErr else { return }
                Unmanaged<Probe>.fromOpaque(ref).takeUnretainedValue().encodedFrame(sample)
            }, refcon: Unmanaged.passUnretained(self).toOpaque(), compressionSessionOut: &encoder), "encoder_create")
        guard let e = encoder else { throw ProbeError.failure("encoder_missing") }
        for (key, value) in [
            (kVTCompressionPropertyKey_RealTime, kCFBooleanTrue as CFTypeRef),
            (kVTCompressionPropertyKey_AllowFrameReordering, kCFBooleanFalse as CFTypeRef),
            (kVTCompressionPropertyKey_ProfileLevel, kVTProfileLevel_H264_ConstrainedBaseline_AutoLevel as CFTypeRef),
            (kVTCompressionPropertyKey_AverageBitRate, 4_000_000 as CFNumber),
            (kVTCompressionPropertyKey_ExpectedFrameRate, 30 as CFNumber),
            (kVTCompressionPropertyKey_MaxKeyFrameInterval, 30 as CFNumber)
        ] { try check(VTSessionSetProperty(e, key: key, value: value), "encoder_property") }
        try check(VTCompressionSessionPrepareToEncodeFrames(e), "encoder_prepare")
        var value: Unmanaged<CFTypeRef>?
        if VTSessionCopyProperty(e, key: kVTCompressionPropertyKey_UsingHardwareAcceleratedVideoEncoder,
            allocator: nil, valueOut: &value) == noErr { hardware = (value?.takeRetainedValue() as? Bool) == true }
    }

    func capture() async throws {
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        let requested = UInt32(option("--display", "0")) ?? 0
        guard let display = content.displays.first(where: { requested == 0 ? $0.displayID == CGMainDisplayID() : $0.displayID == requested })
        else { throw ProbeError.failure("display_not_found") }
        // Preserve aspect ratio, cap at 1080p. Native monitor/DPI acceptance remains manual.
        let scale = min(1.0, min(1920.0 / Double(display.width), 1080.0 / Double(display.height)))
        let w = max(2, Int(Double(display.width) * scale) / 2 * 2)
        let h = max(2, Int(Double(display.height) * scale) / 2 * 2)
        try setupEncoder(width: w, height: h)
        let config = SCStreamConfiguration()
        config.width = w; config.height = h; config.queueDepth = 3
        config.minimumFrameInterval = CMTime(value: 1, timescale: 30)
        config.pixelFormat = kCVPixelFormatType_32BGRA; config.showsCursor = true
        config.capturesAudio = false
        let excluded = content.applications.filter { $0.processID == getpid() }
        let filter = SCContentFilter(display: display, excludingApplications: excluded, exceptingWindows: [])
        let s = SCStream(filter: filter, configuration: config, delegate: self)
        try s.addStreamOutput(self, type: .screen, sampleHandlerQueue: captureQueue)
        stream = s; try await s.startCapture()
    }

    func stream(_ stream: SCStream, didOutputSampleBuffer sample: CMSampleBuffer, of type: SCStreamOutputType) {
        encoderLock.lock(); defer { encoderLock.unlock() }
        guard running, type == .screen, sample.isValid,
            let attachments = CMSampleBufferGetSampleAttachmentsArray(sample, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
            let raw = attachments.first?[.status] as? Int, raw == SCFrameStatus.complete.rawValue,
            let buffer = sample.imageBuffer, let encoder else { return }
        if remote { render(buffer) }
        let code = VTCompressionSessionEncodeFrame(encoder, imageBuffer: buffer,
            presentationTimeStamp: sample.presentationTimeStamp, duration: CMTime(value: 1, timescale: 30),
            frameProperties: nil, sourceFrameRefcon: nil, infoFlagsOut: nil)
        if code != noErr { fail("encode: \(code)") }
    }

    func stream(_ stream: SCStream, didStopWithError error: Error) { fail("capture_stopped: \(error)") }

    func encodedFrame(_ sample: CMSampleBuffer) {
        guard running, let desc = sample.formatDescription, let block = sample.dataBuffer else { return }
        var data = Data(), parameterCount = 0, header: Int32 = 0
        // SPS/PPS accompany each access unit, so dropping stale units is recoverable at the next IDR.
        for index in 0..<2 {
            var ptr: UnsafePointer<UInt8>?, size = 0
            guard CMVideoFormatDescriptionGetH264ParameterSetAtIndex(desc, parameterSetIndex: index,
                parameterSetPointerOut: &ptr, parameterSetSizeOut: &size,
                parameterSetCountOut: &parameterCount, nalUnitHeaderLengthOut: &header) == noErr,
                let ptr else { return }
            data.append(Data([0, 0, 0, 1])); data.append(ptr, count: size)
        }
        guard header == 4 else { fail("unsupported_nal_length"); return }
        let size = CMBlockBufferGetDataLength(block)
        var avcc = Data(count: size)
        let result = avcc.withUnsafeMutableBytes { CMBlockBufferCopyDataBytes(block, atOffset: 0, dataLength: size, destination: $0.baseAddress!) }
        guard result == noErr else { fail("encoded_buffer_copy"); return }
        var p = 0
        while p + 4 <= size {
            let n = number(avcc, p); p += 4
            guard n > 0, n <= size - p else { fail("invalid_encoder_nal"); return }
            data.append(Data([0, 0, 0, 1])); data.append(avcc[p..<p+n]); p += n
        }
        statsLock.lock(); encoded += 1; encodedBytes += data.count; statsLock.unlock()
        if bridge {
            guard data.count <= 4_194_304 else { fail("frame_too_large"); return }
            do { try FileHandle.standardOutput.write(contentsOf: be32(data.count) + data) }
            catch { fail("transport_pipe_closed") }
        } else { decodeQueue.async { self.decode(data) } }
    }

    func readReturnedVideo() {
        decodeQueue.async {
            do {
                while self.running {
                    let count = number(try readExact(4), 0)
                    guard count > 0, count <= 4_194_304 else { throw ProbeError.failure("invalid_frame_length") }
                    self.decode(try readExact(count))
                }
            } catch { if self.running { self.fail("transport_closed") } }
        }
    }

    func decode(_ data: Data) {
        decoderLock.lock(); defer { decoderLock.unlock() }
        guard running else { return }
        // Annex B parsing is bounded by the frame envelope; accept 3- and 4-byte start codes.
        let b = [UInt8](data); var starts: [(Int, Int)] = []; var i = 0
        while i + 2 < b.count {
            if b[i] == 0 && b[i+1] == 0 {
                if b[i+2] == 1 { starts.append((i, i+3)); i += 3; continue }
                if i + 3 < b.count && b[i+2] == 0 && b[i+3] == 1 { starts.append((i, i+4)); i += 4; continue }
            }; i += 1
        }
        var avcc = Data(); var nextSps = sps, nextPps = pps
        for (index, pair) in starts.enumerated() {
            let end = index+1 < starts.count ? starts[index+1].0 : b.count
            guard pair.1 < end else { continue }
            let nal = Data(b[pair.1..<end]), kind = b[pair.1] & 31
            if kind == 7 { nextSps = nal } else if kind == 8 { nextPps = nal }
            else { avcc.append(be32(nal.count)); avcc.append(nal) }
        }
        do {
            if decoder == nil || nextSps != sps || nextPps != pps {
                guard !nextSps.isEmpty, !nextPps.isEmpty else { return }
                if let decoder { VTDecompressionSessionInvalidate(decoder) }; decoder = nil
                sps = nextSps; pps = nextPps
                try sps.withUnsafeBytes { s in try pps.withUnsafeBytes { p in
                    let pointers = [s.baseAddress!.assumingMemoryBound(to: UInt8.self), p.baseAddress!.assumingMemoryBound(to: UInt8.self)]
                    let sizes = [sps.count, pps.count]
                    try check(CMVideoFormatDescriptionCreateFromH264ParameterSets(allocator: nil,
                        parameterSetCount: 2, parameterSetPointers: pointers, parameterSetSizes: sizes,
                        nalUnitHeaderLength: 4, formatDescriptionOut: &format), "decode_format")
                } }
                var cb = VTDecompressionOutputCallbackRecord(decompressionOutputCallback: { ref, _, code, _, image, _, _ in
                    guard code == noErr, let ref, let image else { return }
                    Unmanaged<Probe>.fromOpaque(ref).takeUnretainedValue().render(image)
                }, decompressionOutputRefCon: Unmanaged.passUnretained(self).toOpaque())
                try check(VTDecompressionSessionCreate(allocator: nil, formatDescription: format!, decoderSpecification: nil,
                    imageBufferAttributes: [kCVPixelBufferPixelFormatTypeKey: kCVPixelFormatType_32BGRA] as CFDictionary,
                    outputCallback: &cb, decompressionSessionOut: &decoder), "decoder_create")
            }
            guard !avcc.isEmpty, let decoder, let format else { return }
            var block: CMBlockBuffer?, sample: CMSampleBuffer?
            try check(CMBlockBufferCreateWithMemoryBlock(allocator: nil, memoryBlock: nil, blockLength: avcc.count,
                blockAllocator: nil, customBlockSource: nil, offsetToData: 0, dataLength: avcc.count, flags: 0,
                blockBufferOut: &block), "decode_block")
            try avcc.withUnsafeBytes { try check(CMBlockBufferReplaceDataBytes(with: $0.baseAddress!, blockBuffer: block!, offsetIntoDestination: 0, dataLength: avcc.count), "decode_copy") }
            var size = avcc.count
            try check(CMSampleBufferCreateReady(allocator: nil, dataBuffer: block, formatDescription: format,
                sampleCount: 1, sampleTimingEntryCount: 0, sampleTimingArray: nil,
                sampleSizeEntryCount: 1, sampleSizeArray: &size, sampleBufferOut: &sample), "decode_sample")
            try check(VTDecompressionSessionDecodeFrame(decoder, sampleBuffer: sample!, flags: [], frameRefcon: nil, infoFlagsOut: nil), "decode_frame")
        } catch { fail(String(describing: error)) }
    }

    func render(_ buffer: CVPixelBuffer) {
        statsLock.lock(); decoded += 1; statsLock.unlock()
        guard rendering.wait(timeout: .now()) == .success else {
            statsLock.lock(); renderDrops += 1; statsLock.unlock(); return
        }
        let ci = CIImage(cvPixelBuffer: buffer)
        let image = context.createCGImage(ci, from: ci.extent)
        DispatchQueue.main.async {
            if self.running { self.preview.layer?.contents = image }
            self.rendering.signal()
        }
    }

    func syntheticFrames() {
        captureQueue.async {
            for n in 0..<90 {
                guard self.running, let encoder = self.encoder else { return }
                var buffer: CVPixelBuffer?
                let code = CVPixelBufferCreate(nil, 320, 180, kCVPixelFormatType_32BGRA,
                    [kCVPixelBufferIOSurfacePropertiesKey: [:]] as CFDictionary, &buffer)
                guard code == kCVReturnSuccess, let buffer else { self.fail("synthetic_buffer"); return }
                CVPixelBufferLockBaseAddress(buffer, [])
                let p = CVPixelBufferGetBaseAddress(buffer)!.assumingMemoryBound(to: UInt8.self)
                let stride = CVPixelBufferGetBytesPerRow(buffer)
                for y in 0..<180 { for x in 0..<320 {
                    let o = y*stride + x*4
                    p[o] = UInt8((x+n)%256); p[o+1] = UInt8((y+n)%256); p[o+2] = UInt8(n%256); p[o+3] = 255
                } }
                CVPixelBufferUnlockBaseAddress(buffer, [])
                self.encoderLock.withLock {
                    if self.running {
                        _ = VTCompressionSessionEncodeFrame(encoder, imageBuffer: buffer,
                            presentationTimeStamp: CMTime(value: Int64(n), timescale: 30), duration: CMTime(value: 1, timescale: 30),
                            frameProperties: nil, sourceFrameRefcon: nil, infoFlagsOut: nil)
                    }
                }
                Thread.sleep(forTimeInterval: 1.0/30)
            }
        }
    }

    func fail(_ reason: String) {
        diagnostic(["result": "failed", "reason": reason])
        DispatchQueue.main.async { self.endWithResult(reason) }
    }
    @objc func end() { endWithResult(running ? nil : "cancelled_before_start") }
    func endWithResult(_ error: String?) {
        guard !finishing else { return }; finishing = true
        running = false; preview.layer?.contents = nil
        Task {
            try? await stream?.stopCapture()
            encoderLock.withLock {
                if let encoder { VTCompressionSessionCompleteFrames(encoder, untilPresentationTimeStamp: .invalid); VTCompressionSessionInvalidate(encoder) }
            }
            decoderLock.withLock {
                if let decoder { VTDecompressionSessionWaitForAsynchronousFrames(decoder); VTDecompressionSessionInvalidate(decoder) }
            }
            let report: [String: Any] = statsLock.withLock { ["schemaVersion": 1, "experiment": bridge ? "local_webrtc_loopback" : (synthetic ? "synthetic_codec" : "native_capture_codec"),
                "result": error == nil && (remote ? encoded > 0 : decoded > 0) ? "passed" : "failed", "reason": error ?? ((remote ? encoded > 0 : decoded > 0) ? "completed" : "no_decoded_frames"),
                "encodedFrames": encoded, "decodedFrames": decoded, "encodedBytes": encodedBytes,
                "renderDrops": renderDrops, "elapsedSeconds": Date().timeIntervalSince(started),
                "width": width, "height": height, "hardwareEncoder": hardware,
                "codec": "H264", "twoDeviceTest": false, "inputTest": false] }
            diagnostic(report)
            let path = option("--report", "")
            if !path.isEmpty { try? json(report).write(to: URL(fileURLWithPath: path), options: .atomic) }
            DispatchQueue.main.async { NSApp.terminate(nil) }
        }
    }
    func windowShouldClose(_ sender: NSWindow) -> Bool { end(); return false }
}

signal(SIGPIPE, SIG_IGN)
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let probe = Probe()
probe.show()
app.run()
