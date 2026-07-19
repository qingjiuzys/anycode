export function formatMoney(
  amountCny: number,
  locale: string = typeof navigator === "undefined" ? "zh-CN" : navigator.language,
  options: Intl.NumberFormatOptions = {},
): string {
  const formatOptions: Intl.NumberFormatOptions = {
    style: "currency",
    currency: "CNY",
    minimumFractionDigits: 2,
    ...options,
  };
  const min = formatOptions.minimumFractionDigits;
  const max = formatOptions.maximumFractionDigits;
  if (
    min != null &&
    max != null &&
    max < min
  ) {
    formatOptions.minimumFractionDigits = max;
  }
  return new Intl.NumberFormat(locale, formatOptions).format(amountCny);
}

export function formatFen(amountFen: number, locale?: string): string {
  return formatMoney(amountFen / 100, locale);
}
