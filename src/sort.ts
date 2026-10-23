// Click-to-sort table columns, shared by the Diagnoses lists.

import { computed, ref, type Ref } from "vue";

export type Direction = "asc" | "desc";

/** A column value to compare; `null` always sorts last. */
export type SortKey<T> = (item: T) => string | number | boolean | null;

export function useSort<T, K extends string>(
  items: Ref<T[]>,
  keys: Record<K, SortKey<T>>,
  initial: NoInfer<K>,
  initialDirection: Direction = "desc",
) {
  const column = ref(initial) as Ref<K>;
  const direction = ref<Direction>(initialDirection);
  const collator = new Intl.Collator(undefined, { sensitivity: "base", numeric: true });

  const sorted = computed(() => {
    const key = keys[column.value];
    const sign = direction.value === "asc" ? 1 : -1;
    return [...items.value].sort((a, b) => {
      const x = key(a);
      const y = key(b);
      if (x === null || y === null) return x === y ? 0 : x === null ? 1 : -1;
      if (typeof x === "string" && typeof y === "string") return sign * collator.compare(x, y);
      return sign * (Number(x) - Number(y));
    });
  });

  /** Sorts by `next`; clicking the current column flips the direction. */
  function sortBy(next: K, defaultDirection: Direction = "desc") {
    if (column.value === next) {
      direction.value = direction.value === "asc" ? "desc" : "asc";
    } else {
      column.value = next;
      direction.value = defaultDirection;
    }
  }

  return { sorted, column, direction, sortBy };
}
