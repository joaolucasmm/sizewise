// The app's top-level sections, in navigation order.

export const SECTIONS = ["disk", "overview", "startup", "processes", "programs", "disks"] as const;

export type Section = (typeof SECTIONS)[number];
