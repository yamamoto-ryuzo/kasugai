import os
from PIL import Image, ImageDraw, ImageFilter

def create_app_icon():
    size = 1024
    
    # ベースの透過キャンバス
    base_img = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    # --- 1. 背景角丸（白〜明るいライトグレー）レイヤー【マージンを小さくして枠いっぱいに】 ---
    bg_layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    bg_draw = ImageDraw.Draw(bg_layer)
    
    # マージンを 64 -> 24 に削減し、アイコンベースを枠いっぱいに拡大
    bg_margin = 24
    bg_x0, bg_y0 = bg_margin, bg_margin
    bg_x1, bg_y1 = size - bg_margin, size - bg_margin
    bg_radius = 200

    # 角丸マスク
    bg_mask = Image.new("L", (size, size), 0)
    bg_mask_draw = ImageDraw.Draw(bg_mask)
    bg_mask_draw.rounded_rectangle([bg_x0, bg_y0, bg_x1, bg_y1], radius=bg_radius, fill=255)

    # 縦方向グラデーション：上部 #FFFFFF (255, 255, 255) -> 下部 #E2E8F0 (226, 232, 240) 
    for y in range(bg_y0, bg_y1):
        ratio = (y - bg_y0) / (bg_y1 - bg_y0)
        r = int(255 * (1 - ratio) + 226 * ratio)
        g = int(255 * (1 - ratio) + 232 * ratio)
        b = int(255 * (1 - ratio) + 240 * ratio)
        bg_draw.line([(bg_x0, y), (bg_x1, y)], fill=(r, g, b, 255))
    
    # マスクを適用してベースに重ねる
    temp_bg = Image.new("RGBA", (size, size), (0,0,0,0))
    temp_bg.paste(bg_layer, (0, 0), mask=bg_mask)
    base_img = Image.alpha_composite(base_img, temp_bg)

    # --- 2. グリッドレイヤー ---
    grid_layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    grid_draw = ImageDraw.Draw(grid_layer)
    grid_spacing = 64
    for i in range(bg_x0, bg_x1, grid_spacing):
        grid_draw.line([(i, bg_y0), (i, bg_y1)], fill=(71, 85, 105, 12))
    for j in range(bg_y0, bg_y1, grid_spacing):
        grid_draw.line([(bg_x0, j), (bg_x1, j)], fill=(71, 85, 105, 12))

    temp_grid = Image.new("RGBA", (size, size), (0,0,0,0))
    temp_grid.paste(grid_layer, (0, 0), mask=bg_mask)
    base_img = Image.alpha_composite(base_img, temp_grid)

    # --- 3. ペイン（3画面）レイヤー【大幅に縦横拡大】 ---
    pane_layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    pane_draw = ImageDraw.Draw(pane_layer)

    # ペインのサイズを大幅に大きく
    pane_w = 240  # 190 -> 240
    pane_h = 580  # 440 -> 580 (縦いっぱい)
    pane_gap = 48  # 40 -> 48
    start_x = (size - (pane_w * 3 + pane_gap * 2)) // 2  # (1024 - (720 + 96)) // 2 = 104
    start_y = 220  # 300 -> 220 (上寄りに配置)
    pane_radius = 32

    pane_colors = [
        {"fill": (14, 165, 233, 45), "stroke": (14, 165, 233, 255), "glow": (14, 165, 233, 30)},   # Pane 1 (Portal Blue)
        {"fill": (16, 185, 129, 45), "stroke": (16, 185, 129, 255), "glow": (16, 185, 129, 30)},   # Pane 2 (RAG Green)
        {"fill": (139, 92, 246, 45), "stroke": (139, 92, 246, 255), "glow": (139, 92, 246, 30)}   # Pane 3 (AI Purple)
    ]

    # グロー（ぼかし用の光彩）
    glow_layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    glow_draw = ImageDraw.Draw(glow_layer)
    for i in range(3):
        x0 = start_x + i * (pane_w + pane_gap)
        y0 = start_y
        x1 = x0 + pane_w
        y1 = y0 + pane_h
        glow_draw.rounded_rectangle([x0 - 20, y0 - 20, x1 + 20, y1 + 20], radius=pane_radius + 20, fill=pane_colors[i]["glow"])
    
    glow_layer = glow_layer.filter(ImageFilter.GaussianBlur(18))
    base_img = Image.alpha_composite(base_img, glow_layer)

    # 実体ペイン
    for i in range(3):
        x0 = start_x + i * (pane_w + pane_gap)
        y0 = start_y
        x1 = x0 + pane_w
        y1 = y0 + pane_h
        
        # 塗りつぶし
        pane_draw.rounded_rectangle([x0, y0, x1, y1], radius=pane_radius, fill=pane_colors[i]["fill"])
        # 枠線
        pane_draw.rounded_rectangle([x0, y0, x1, y1], radius=pane_radius, outline=pane_colors[i]["stroke"], width=8)  # 枠太さも 6 -> 8

        # 画面内部のダミーグリッドライン
        inner_y = y0 + 60
        while inner_y < y1 - 50:
            pane_draw.line([(x0 + 40, inner_y), (x1 - 40, inner_y)], fill=(100, 116, 139, 40), width=4)
            inner_y += 75

    base_img = Image.alpha_composite(base_img, pane_layer)

    # --- 4. 鎹（かすがい）レイヤー【枠いっぱいに超特大化】 ---
    k_layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    k_draw = ImageDraw.Draw(k_layer)

    # 圧倒的なシンボルサイズに再設計
    kasugai_y = start_y + 130
    kasugai_h = 120  # 横棒の太さを 96 -> 120 に大幅拡大（極太クランプ）
    k_left_x = start_x + pane_w // 2
    k_right_x = start_x + pane_w * 2 + pane_gap * 2 + pane_w // 2
    claw_w = 120  # 爪の太さを 96 -> 120 に大幅拡大
    claw_h = 320  # 爪の高さを 230 -> 320 に超大幅拡大（画面の半分以上を強烈に結合）

    # 鎹のシャドウ（超大型化に合わせてダイナミックな3D陰影に）
    shadow_layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    shadow_draw = ImageDraw.Draw(shadow_layer)
    shadow_offset = 22
    # 横棒と左右爪の影
    shadow_draw.rounded_rectangle([k_left_x - claw_w//2, kasugai_y + shadow_offset, k_right_x + claw_w//2, kasugai_y + kasugai_h + shadow_offset], radius=24, fill=(15, 23, 42, 100))
    shadow_draw.rounded_rectangle([k_left_x - claw_w//2, kasugai_y + shadow_offset, k_left_x + claw_w//2, kasugai_y + claw_h + shadow_offset], radius=24, fill=(15, 23, 42, 100))
    shadow_draw.rounded_rectangle([k_right_x - claw_w//2, kasugai_y + shadow_offset, k_right_x + claw_w//2, kasugai_y + claw_h + shadow_offset], radius=24, fill=(15, 23, 42, 100))
    
    shadow_layer = shadow_layer.filter(ImageFilter.GaussianBlur(25))
    base_img = Image.alpha_composite(base_img, shadow_layer)

    # 鎹本体の描画関数
    def draw_metallic_rect(draw_obj, rect, is_vertical=False):
        x0, y0, x1, y1 = rect
        if is_vertical:
            for x in range(int(x0), int(x1)):
                ratio = (x - x0) / (x1 - x0)
                factor = 1.0 - abs(ratio - 0.5) * 2.0
                r = int(215 + 40 * factor)
                g = int(125 + 115 * factor)
                b = int(5 + 125 * factor)
                draw_obj.line([(x, y0), (x, y1)], fill=(r, g, b, 255))
        else:
            for y in range(int(y0), int(y1)):
                ratio = (y - y0) / (y1 - y0)
                factor = 1.0 - abs(ratio - 0.3) * 1.5
                factor = max(0.0, min(1.0, factor))
                r = int(215 + 40 * factor)
                g = int(125 + 115 * factor)
                b = int(5 + 125 * factor)
                draw_obj.line([(x0, y), (x1, y)], fill=(r, g, b, 255))

    # 左爪（縦）
    draw_metallic_rect(k_draw, [k_left_x - claw_w//2, kasugai_y, k_left_x + claw_w//2, kasugai_y + claw_h], is_vertical=True)
    # 右爪（縦）
    draw_metallic_rect(k_draw, [k_right_x - claw_w//2, kasugai_y, k_right_x + claw_w//2, kasugai_y + claw_h], is_vertical=True)
    # メインバー（横）
    draw_metallic_rect(k_draw, [k_left_x - claw_w//2, kasugai_y, k_right_x + claw_w//2, kasugai_y + kasugai_h], is_vertical=False)

    # リベット（留め具もサイズアップ：24 -> 32）
    def draw_rivet(draw_obj, cx, cy, r):
        draw_obj.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(110, 50, 0, 255))
        draw_obj.ellipse([cx - r + 5, cy - r + 5, cx + r - 5, cy + r - 5], fill=(255, 245, 150, 255))
        draw_obj.ellipse([cx - r + 10, cy - r + 10, cx + r - 10, cy + r - 10], fill=(170, 70, 0, 255))

    draw_rivet(k_draw, k_left_x, kasugai_y + kasugai_h//2, 32)
    draw_rivet(k_draw, k_right_x, kasugai_y + kasugai_h//2, 32)

    # 爪の先端の三角形（太さに合わせ大きく鋭利に）
    claw_tip_h = 60
    # 左爪の先
    k_draw.polygon([
        (k_left_x - claw_w//2, kasugai_y + claw_h),
        (k_left_x + claw_w//2, kasugai_y + claw_h),
        (k_left_x, kasugai_y + claw_h + claw_tip_h)
    ], fill=(170, 70, 0, 255))
    k_draw.polygon([
        (k_left_x - claw_w//4, kasugai_y + claw_h),
        (k_left_x + claw_w//4, kasugai_y + claw_h),
        (k_left_x, kasugai_y + claw_h + claw_tip_h)
    ], fill=(245, 158, 11, 255))

    # 右爪の先
    k_draw.polygon([
        (k_right_x - claw_w//2, kasugai_y + claw_h),
        (k_right_x + claw_w//2, kasugai_y + claw_h),
        (k_right_x, kasugai_y + claw_h + claw_tip_h)
    ], fill=(170, 70, 0, 255))
    k_draw.polygon([
        (k_right_x - claw_w//4, kasugai_y + claw_h),
        (k_right_x + claw_w//4, kasugai_y + claw_h),
        (k_right_x, kasugai_y + claw_h + claw_tip_h)
    ], fill=(245, 158, 11, 255))

    # フチのハイライト線（太さ 6px -> 8px にアップして重厚な金属性をアピール）
    k_draw.line([(k_left_x - claw_w//2, kasugai_y), (k_right_x + claw_w//2, kasugai_y)], fill=(255, 255, 255, 220), width=8)
    k_draw.line([(k_left_x - claw_w//2, kasugai_y), (k_left_x - claw_w//2, kasugai_y + claw_h)], fill=(255, 255, 255, 220), width=8)
    k_draw.line([(k_right_x + claw_w//2, kasugai_y), (k_right_x + claw_w//2, kasugai_y + claw_h)], fill=(255, 255, 255, 140), width=8)

    base_img = Image.alpha_composite(base_img, k_layer)

    # --- 5. エネルギーリンク（弧）レイヤー ---
    link_layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    link_draw = ImageDraw.Draw(link_layer)
    # 接続エネルギーの弧も太く力強く（width 10 -> 12）
    link_draw.arc([k_left_x, kasugai_y + 120, start_x + pane_w + pane_gap + pane_w//2, kasugai_y + 350], start=180, end=270, fill=(14, 165, 233, 210), width=12)
    link_draw.arc([start_x + pane_w + pane_gap + pane_w//2, kasugai_y + 120, k_right_x, kasugai_y + 350], start=270, end=360, fill=(139, 92, 246, 210), width=12)
    
    link_layer = link_layer.filter(ImageFilter.GaussianBlur(6))
    base_img = Image.alpha_composite(base_img, link_layer)

    # --- 6. ライトオーバーレイレイヤー ---
    light_layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    light_draw = ImageDraw.Draw(light_layer)
    for r_idx in range(0, 1000, 20):
        alpha = int(25 * (1 - r_idx / 1000))
        light_draw.ellipse([bg_x0 - r_idx, bg_y0 - r_idx, bg_x0 + r_idx, bg_y0 + r_idx], outline=(255, 255, 255, alpha), width=10)
    
    temp_light = Image.new("RGBA", (size, size), (0,0,0,0))
    temp_light.paste(light_layer, (0, 0), mask=bg_mask)
    base_img = Image.alpha_composite(base_img, temp_light)

    # 最終的な画像を切り抜く
    final_img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    final_img.paste(base_img, (0, 0), mask=bg_mask)

    # 保存
    os.makedirs("images", exist_ok=True)
    final_img.save("images/kasugai_new_app_icon.png")
    os.makedirs("kasugai/src-tauri", exist_ok=True)
    final_img.save("kasugai/app-icon.png")
    print("Icon filled with massive elements generated successfully!")

if __name__ == "__main__":
    create_app_icon()
