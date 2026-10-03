package dev.opencut.viewertexture

import java.nio.ByteBuffer

class AndroidViewerFrame(
    val pixels: ByteBuffer,
    val width: Int,
    val height: Int,
    val releaseContext: Long,
    val generation: Long,
)
