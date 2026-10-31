/**
 * 根据网卡 IPv4 地址推导完整私有地址范围。
 *
 * @author lhy
 */
/**
 * 解析带前缀的网卡 IPv4 地址并返回完整私有网段，无法识别时返回空字符串。
 *
 * @param address 带前缀长度的 IPv4 地址
 * @returns 识别出的私有 CIDR，无法识别时为空字符串
 * @author lhy
 */
export function privateNetworkFromAddress(address: string): string {
  const match = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\/(\d{1,2})$/.exec(
    address,
  );
  if (!match) return "";
  const octets = match.slice(1, 5).map(Number);
  const prefix = Number(match[5]);
  if (octets.some((value) => value > 255) || prefix < 1 || prefix > 32)
    return "";
  if (octets[0] === 10) return "10.0.0.0/8";
  if (octets[0] === 172 && octets[1] >= 16 && octets[1] <= 31)
    return "172.16.0.0/12";
  if (octets[0] === 192 && octets[1] === 168) return "192.168.0.0/16";
  return "";
}
