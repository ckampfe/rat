export function bytes_to_object_url(slice, mimeType) {
  const blob = new Blob([slice], { type: mimeType });
  const imageUrl = URL.createObjectURL(blob);
  return imageUrl;
};
