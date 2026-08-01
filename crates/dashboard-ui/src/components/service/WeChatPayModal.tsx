import { useEffect, useRef, useState } from "react";
import { accountCloud } from "@/api/client/accountCloud";
import type { PaymentOrder } from "@/api/types/accountCloud";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { useT } from "@/i18n/context";

type Props = {
  baseUrl: string;
  order: PaymentOrder;
  onClose: () => void;
  onPaid: () => void;
};

export function WeChatPayModal({ baseUrl, order, onClose, onPaid }: Props) {
  const t = useT();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [status, setStatus] = useState(order.status);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!order.code_url || !canvasRef.current) return;
    void import("qrcode").then((QRCode) => {
      void QRCode.toCanvas(canvasRef.current, order.code_url!, {
        width: 220,
        margin: 1,
      });
    });
  }, [order.code_url]);

  useEffect(() => {
    if (status === "paid") return;
    let ticks = 0;
    const timer = window.setInterval(() => {
      ticks += 1;
      const poll =
        ticks % 4 === 0
          ? accountCloud.syncPaymentOrder(baseUrl, order.id)
          : accountCloud.getPaymentOrder(baseUrl, order.id);
      void poll
        .then((res) => {
          setStatus(res.order.status);
          if (res.order.status === "paid") {
            onPaid();
          }
        })
        .catch((e: unknown) => {
          setError(e instanceof Error ? e.message : String(e));
        });
    }, 2500);
    return () => window.clearInterval(timer);
  }, [baseUrl, order.id, onPaid, status]);

  const amountYuan = (order.amount_fen / 100).toFixed(2);

  return (
    <ModalOverlay open labelledBy="wechat-pay-title" onClose={onClose} zIndex={380}>
      <div className="glass-modal rounded-xl p-6 max-w-sm w-full console-pay-modal">
        <h2 id="wechat-pay-title" className="text-lg font-semibold m-0 mb-2">
          {t("service.plan.wechatPayTitle")}
        </h2>
        <p className="text-sm text-secondary m-0 mb-4">
          {t("service.plan.wechatPayHint")
            .replace("{plan}", order.plan)
            .replace("{amount}", amountYuan)}
        </p>
        {order.code_url && status === "pending" && (
          <div className="console-pay-qr-wrap">
            <canvas ref={canvasRef} aria-label={t("service.plan.wechatQrAlt")} />
          </div>
        )}
        {status === "paid" && (
          <p className="text-sm text-success m-0 mb-3">{t("service.plan.wechatPaid")}</p>
        )}
        {status === "pending" && (
          <p className="text-xs text-secondary m-0 mb-3">{t("service.plan.wechatWaiting")}</p>
        )}
        {error && (
          <p className="text-sm text-error m-0 mb-3" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end">
          <button type="button" className="dw-btn-secondary text-sm" onClick={onClose}>
            {t("service.plan.wechatClose")}
          </button>
        </div>
      </div>
    </ModalOverlay>
  );
}
