import * as React from "react";
import { Triangle, Hash } from "lucide-react";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { RichViolationCounts } from "@/lib/ipc";
import { cn } from "@/lib/utils";

/**
 * Full violation badges with category icon + count. Designed for
 * inline rows where horizontal space is plentiful (palette results,
 * flat list, status bar).
 *
 * Colors come from semantic tokens defined in `index.css`
 * (`--badge-naming` / `--badge-placement` / `--badge-sequence`) so
 * dark mode swaps one variable instead of every utility — see
 * `customization.md §Dark Mode` in the shadcn skill for the rule.
 *
 * Optional `titles` wraps each badge in a shadcn `<Tooltip>` whose
 * content is the supplied (often multi-line) string. Status bar
 * uses this to surface the contributing files on hover. Callers
 * that don't pass `titles` get a plain badge — no Tooltip wrapper,
 * no behavioral cost.
 */
export function ViolationBadges({
  counts,
  titles,
  className,
}: {
  counts: RichViolationCounts;
  titles?: { naming?: string; placement?: string; sequence?: string };
  /** Override the wrapper layout. Defaults to `ml-auto` (right-aligned
   *  inside flex parents like the palette / flat view rows). The
   *  inspector passes its own classes so the badges sit at the start
   *  of the value column instead of being pushed to the cell edge. */
  className?: string;
}) {
  const total = counts.naming + counts.placement + counts.sequence;
  if (total === 0) return null;
  return (
    <span
      className={cn(
        "flex items-center gap-1 text-[0.625rem] tracking-wide",
        className ?? "ml-auto",
      )}
    >
      {counts.naming > 0 ? (
        <Badge tooltip={titles?.naming}>
          <span className="rounded bg-badge-naming/15 px-1 py-0.5 text-badge-naming">
            <Triangle className="inline size-2.5" /> {counts.naming}
          </span>
        </Badge>
      ) : null}
      {counts.placement > 0 ? (
        <Badge tooltip={titles?.placement}>
          <span className="rounded bg-badge-placement/15 px-1 py-0.5 text-badge-placement">
            <Hash className="inline size-2.5" /> {counts.placement}
          </span>
        </Badge>
      ) : null}
      {counts.sequence > 0 ? (
        <Badge tooltip={titles?.sequence}>
          <span className="rounded bg-badge-sequence/15 px-1 py-0.5 text-badge-sequence">
            ≡ {counts.sequence}
          </span>
        </Badge>
      ) : null}
    </span>
  );
}

/**
 * Wrap `children` in a shadcn Tooltip when `tooltip` is non-empty,
 * otherwise pass `children` straight through. Keeps the badge JSX
 * above linear and avoids spinning up a Tooltip portal for every
 * row in dense lists where no tooltip is configured.
 */
function Badge({ tooltip, children }: { tooltip: string | undefined; children: React.ReactNode }) {
  if (!tooltip) return <>{children}</>;
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent className="max-w-md font-mono whitespace-pre-line text-[0.625rem]">
        {tooltip}
      </TooltipContent>
    </Tooltip>
  );
}

/**
 * Compact dot indicator used in the tree view where the rows are
 * dense and an icon-and-count badge would crowd out the filename.
 */
export function ViolationDots({ counts }: { counts: RichViolationCounts }) {
  const total = counts.naming + counts.placement + counts.sequence;
  if (total === 0) return null;
  return (
    <span className="ml-1 flex items-center gap-0.5">
      {counts.naming > 0 ? <span className="size-1.5 rounded-full bg-badge-naming" /> : null}
      {counts.placement > 0 ? <span className="size-1.5 rounded-full bg-badge-placement" /> : null}
      {counts.sequence > 0 ? <span className="size-1.5 rounded-full bg-badge-sequence" /> : null}
    </span>
  );
}
