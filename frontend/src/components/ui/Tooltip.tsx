import { Tooltip as BaseTooltip } from '@base-ui-components/react/tooltip';
import type { ReactElement, ReactNode } from 'react';

/** Wrap the app once so all tooltips share timing. */
export function TooltipProvider({ children }: { children: ReactNode }): React.JSX.Element {
  return <BaseTooltip.Provider delay={400} closeDelay={80}>{children}</BaseTooltip.Provider>;
}

interface TooltipProps {
  label: ReactNode;
  /** Single interactive element to attach the tooltip to. */
  children: ReactElement;
}

export function Tooltip({ label, children }: TooltipProps): React.JSX.Element {
  return (
    <BaseTooltip.Root>
      <BaseTooltip.Trigger render={children} />
      <BaseTooltip.Portal>
        <BaseTooltip.Positioner sideOffset={6}>
          <BaseTooltip.Popup className="app-tooltip">
            {label}
            <BaseTooltip.Arrow className="app-tooltip-arrow" />
          </BaseTooltip.Popup>
        </BaseTooltip.Positioner>
      </BaseTooltip.Portal>
    </BaseTooltip.Root>
  );
}
