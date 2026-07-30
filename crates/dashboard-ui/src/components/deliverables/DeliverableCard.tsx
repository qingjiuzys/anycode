import {
  selectDeliverableViewer,
  type DeliverableCardProps,
} from "@/lib/selectDeliverableViewer";

export type { DeliverableCardProps };

export function DeliverableCard(props: DeliverableCardProps) {
  return selectDeliverableViewer({ ...props, variant: "compact" });
}
