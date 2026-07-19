export function formatMoney(amountFen: number, locale: string = "zh-CN"): string {
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency: "CNY",
    minimumFractionDigits: 2,
  }).format(amountFen / 100);
}
