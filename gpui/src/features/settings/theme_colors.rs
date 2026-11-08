use super::*;

// ── Shared pieces ────────────────────────────────────────────────────────

pub(crate) struct ColorField {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
    pub(crate) description: &'static str,
    /// The source's default for this colour. Only the Cut tool colours still
    /// read it — the theme colours come from `gpui_store::ThemeColors`, and
    /// this value records what the source module ships for them.
    pub(crate) placeholder: &'static str,
}

pub(crate) struct ColorSection {
    pub(crate) title: &'static str,
    pub(crate) description: &'static str,
    pub(crate) rows: &'static [ColorField],
}

pub(crate) static FOUNDATION_ROWS: [ColorField; 5] = [
    ColorField { key: "background", label: "App background", description: "Outer app frame and page background.", placeholder: "#0b0f14" },
    ColorField { key: "foreground", label: "App foreground", description: "Top-level body text fallback color.", placeholder: "#e6ebf2" },
    ColorField { key: "surface0", label: "Surface 0", description: "Main modal and panel base surface.", placeholder: "#111216" },
    ColorField { key: "surface1", label: "Surface 1", description: "Raised cards, tool panes, and section fills.", placeholder: "#1a1b21" },
    ColorField { key: "surface2", label: "Surface 2", description: "Secondary tiles and inset controls.", placeholder: "#23252e" },
];

pub(crate) static CONTENT_ROWS: [ColorField; 5] = [
    ColorField { key: "textStrong", label: "Text strong", description: "Primary headings and high-contrast labels.", placeholder: "#f8f8fb" },
    ColorField { key: "textMuted", label: "Text muted", description: "Supporting labels, hints, and metadata.", placeholder: "#c3c7cf" },
    ColorField { key: "indicator", label: "Indicator", description: "Neutral dots, markers, and status indicators.", placeholder: "#c3c7cf" },
    ColorField { key: "borderSubtle", label: "Border subtle", description: "Low-contrast panel and input outlines.", placeholder: "#272a33" },
    ColorField { key: "borderStrong", label: "Border strong", description: "Higher-contrast structural dividers.", placeholder: "#353944" },
];

pub(crate) static PRIMARY_BRAND_ROWS: [ColorField; 5] = [
    ColorField { key: "accent", label: "Accent", description: "Primary highlight, active icons, and focus color.", placeholder: "#ec2a77" },
    ColorField { key: "accentHover", label: "Accent hover", description: "Hover/pressed state for primary accent actions.", placeholder: "#d81d67" },
    ColorField { key: "primaryButtonSurface", label: "Primary button surface", description: "Filled primary buttons and pills.", placeholder: "#c11f61" },
    ColorField { key: "accentContrast", label: "Accent contrast", description: "Text/icons shown on primary accent fills.", placeholder: "#fff6ff" },
    ColorField { key: "topbarAccent", label: "Top bar accent", description: "Glow and accent wash used by the app bar.", placeholder: "#ec2a77" },
];

pub(crate) static SECONDARY_BRAND_ROWS: [ColorField; 4] = [
    ColorField { key: "accentSecondary", label: "Accent secondary", description: "Secondary accent and approved-action color.", placeholder: "#baf72e" },
    ColorField { key: "accentSecondaryHover", label: "Secondary hover", description: "Hover/pressed state for green actions.", placeholder: "#a6df29" },
    ColorField { key: "secondaryButtonSurface", label: "Secondary button surface", description: "Filled secondary buttons and badges.", placeholder: "#9bcc26" },
    ColorField { key: "accentSecondaryContrast", label: "Secondary contrast", description: "Text/icons shown on green fills.", placeholder: "#182106" },
];

pub(crate) static SCENE_ROWS: [ColorField; 3] = [
    ColorField { key: "sceneGradientRadial", label: "3D radial glow", description: "Radial color bloom in the scene backdrop.", placeholder: "#ff37aa" },
    ColorField { key: "sceneGradientLinearStart", label: "3D gradient top", description: "Top stop of the linear scene gradient.", placeholder: "#ff37aa" },
    ColorField { key: "sceneGradientLinearMid", label: "3D gradient mid", description: "Middle stop of the linear scene gradient.", placeholder: "#6f33ff" },
];

pub(crate) static STATUS_ROWS: [ColorField; 2] = [
    ColorField { key: "success", label: "Success", description: "Success states, confirmations, and healthy signals.", placeholder: "#2eb67d" },
    ColorField { key: "danger", label: "Danger", description: "Destructive actions, warnings, and errors.", placeholder: "#e45454" },
];

pub(crate) static THEME_COLOR_SECTIONS: [ColorSection; 6] = [
    ColorSection { title: "Foundation surfaces", description: "Core app backgrounds and panel surfaces.", rows: &FOUNDATION_ROWS },
    ColorSection { title: "Content contrast", description: "Typography, borders, and neutral UI signals.", rows: &CONTENT_ROWS },
    ColorSection { title: "Primary brand accent", description: "Primary action styling, highlights, and key brand colors.", rows: &PRIMARY_BRAND_ROWS },
    ColorSection { title: "Secondary brand accent", description: "Secondary action styling, complementary highlights, and secondary brand colors.", rows: &SECONDARY_BRAND_ROWS },
    ColorSection { title: "Scene accents", description: "3D view gradient treatment and scene chrome.", rows: &SCENE_ROWS },
    ColorSection { title: "Semantic status colors", description: "Feedback colors for success and destructive actions.", rows: &STATUS_ROWS },
];

pub(crate) static MESH_HIGHLIGHT_COLOR_FIELDS: [ColorField; 2] = [
    ColorField { key: "meshSelectionColor", label: "Selection color", description: "Tint applied to selected models.", placeholder: "#ec2a77" },
    ColorField { key: "meshHoverColor", label: "Hover color", description: "Tint applied to hovered models.", placeholder: "#ec2a77" },
];

pub(crate) static CUT_COLOR_FIELDS: [ColorField; 16] = [
    ColorField { key: "seam", label: "Seam", description: "The cut line you draw.", placeholder: "#00008b" },
    ColorField { key: "seamHover", label: "Seam (hover)", description: "The seam while the cursor is over it.", placeholder: "#c9752e" },
    ColorField { key: "seamInactive", label: "Seam (other loops)", description: "Loops of a multi-loop cut you are not editing.", placeholder: "#5e3418" },
    ColorField { key: "seamGlow", label: "Seam glow", description: "Halo around the hovered seam.", placeholder: "#ffe6cc" },
    ColorField { key: "cutSurface", label: "Cut surface", description: "The contour membrane and the flat cut plane.", placeholder: "#37ff7a" },
    ColorField { key: "tenonFront", label: "Tenon (near faces)", description: "The tenon's faces turned toward you.", placeholder: "#001e57" },
    ColorField { key: "tenonBack", label: "Tenon (far faces)", description: "Far faces, darker so the shape reads solid.", placeholder: "#8a4a08" },
    ColorField { key: "tenonEdge", label: "Tenon edges", description: "The tenon's silhouette lines.", placeholder: "#ff7a00" },
    ColorField { key: "mortiseFront", label: "Mortise (near faces)", description: "The hole carved in the other half.", placeholder: "#c07bff" },
    ColorField { key: "mortiseBack", label: "Mortise (far faces)", description: "Far faces, darker so the shape reads solid.", placeholder: "#4a2a7a" },
    ColorField { key: "mortiseEdge", label: "Mortise edges", description: "The mortise's silhouette lines.", placeholder: "#a24bff" },
    ColorField { key: "tenonHandle", label: "Tenon handle", description: "The dot you drag to slide the tenon.", placeholder: "#0091ff" },
    ColorField { key: "markerFirst", label: "First waypoint", description: "The point the loop starts from.", placeholder: "#37ff7a" },
    ColorField { key: "markerPoint", label: "Waypoint", description: "Every other point on the loop.", placeholder: "#ffd24a" },
    ColorField { key: "markerSelected", label: "Waypoint (selected)", description: "The point you clicked.", placeholder: "#0091ff" },
    ColorField { key: "markerDragging", label: "Waypoint (dragging)", description: "The point being dragged.", placeholder: "#35e3ff" },
];
