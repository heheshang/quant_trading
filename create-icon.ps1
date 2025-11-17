Add-Type -AssemblyName System.Drawing

# Create a 512x512 bitmap
$bmp = New-Object System.Drawing.Bitmap 512, 512

# Create graphics object
$graphics = [System.Drawing.Graphics]::FromImage($bmp)

# Fill background with a gradient
$brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    [System.Drawing.Point]::new(0, 0),
    [System.Drawing.Point]::new(512, 512),
    [System.Drawing.Color]::FromArgb(41, 128, 185),
    [System.Drawing.Color]::FromArgb(52, 73, 94)
)
$graphics.FillRectangle($brush, 0, 0, 512, 512)

# Draw a simple "Q" letter for Quant
$font = New-Object System.Drawing.Font("Arial", 280, [System.Drawing.FontStyle]::Bold)
$textBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$stringFormat = New-Object System.Drawing.StringFormat
$stringFormat.Alignment = [System.Drawing.StringAlignment]::Center
$stringFormat.LineAlignment = [System.Drawing.StringAlignment]::Center

$graphics.DrawString("Q", $font, $textBrush, 256, 256, $stringFormat)

# Save the image
$bmp.Save("$PSScriptRoot\app-icon.png", [System.Drawing.Imaging.ImageFormat]::Png)

# Cleanup
$graphics.Dispose()
$bmp.Dispose()
$brush.Dispose()
$textBrush.Dispose()
$font.Dispose()

Write-Host "Icon created successfully: app-icon.png"
