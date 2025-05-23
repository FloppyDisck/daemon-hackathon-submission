module daemon::palette;

use sui::random::RandomGenerator;

public struct MiniColor has store, copy, drop {
    hue: u16,
    saturation: u8,
    value: u8,
}

public struct MiniPalette has store, copy, drop {
    primary: MiniColor,
    accent: MiniColor,
    highlight: MiniColor,
    neutral: MiniColor,
    background: MiniColor,
}

public(package) fun generate_palette(generator: &mut RandomGenerator): MiniPalette {
    let primary_hue = generator.generate_u16_in_range(0, 360);
    let primary = new_color(
        primary_hue,
        generator.generate_u8_in_range(102, 242),
        generator.generate_u8_in_range(204, 255)
    );

    let background = new_color(
        shift_hue(primary_hue, 180, false),
        generator.generate_u8_in_range(76, 140),
        generator.generate_u8_in_range(153, 204)
    );

    let highlight = new_color(
        shift_hue(
            primary_hue,
            generator.generate_u16_in_range(30, 60),
            generator.generate_bool()
        ),
        generator.generate_u8_in_range(102, 242),
        generator.generate_u8_in_range(204, 255)
    );

    let accent = new_color(
        shift_hue(
            primary_hue,
            15,
            generator.generate_bool()
        ),
        generator.generate_u8_in_range(204, 255),
        generator.generate_u8_in_range(64, 115)
    );

    let neutral = new_color(
        primary_hue,
        generator.generate_u8_in_range(20, 64),
        generator.generate_u8_in_range(179, 230)
    );

    MiniPalette {
        primary,
        background,
        highlight,
        accent,
        neutral,
    }
}

public fun new_color(hue: u16, saturation: u8, value: u8): MiniColor {
    MiniColor {
        hue,
        saturation,
        value,
    }
}

fun shift_hue(original_hue: u16, shift_amount: u16, is_negative: bool): u16 {
    // Handle positive shifts
    if (!is_negative) {
        // Add shift and wrap around using modulo
        (original_hue + shift_amount) % 360
    }
    // Handle negative shifts
    else if (shift_amount > original_hue) {
        // If we need to wrap around, subtract from 360
        360 - (shift_amount - original_hue) % 360
    } else {
        // Simple subtraction if we don't cross zero
        original_hue - shift_amount
    }
}
