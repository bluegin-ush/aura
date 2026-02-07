const API = 'http://localhost:8081';

export const api = {
  // Parts
  getParts: () => fetch(`${API}/parts`).then(r => r.json()),
  searchParts: (q) => fetch(`${API}/partsSearch?q=${encodeURIComponent(q)}`).then(r => r.json()),
  createPart: (data) => fetch(`${API}/part/${enc(data.code)}/${enc(data.name)}/${enc(data.brand || 'N/A')}/${data.price}/${data.stock || 0}/${data.min_stock || 5}`, { method: 'POST' }).then(r => r.json()),
  deletePart: (id) => fetch(`${API}/part/${id}`, { method: 'DELETE' }).then(r => r.json()),

  // Motos
  getMotos: () => fetch(`${API}/motos`).then(r => r.json()),
  createMoto: (data) => fetch(`${API}/moto/${enc(data.plate)}/${enc(data.brand)}/${enc(data.model)}/${data.year}/${enc(data.owner_name)}/${enc(data.owner_phone || 'N/A')}`, { method: 'POST' }).then(r => r.json()),
  deleteMoto: (id) => fetch(`${API}/moto/${id}`, { method: 'DELETE' }).then(r => r.json()),
  getMotoOrders: (id) => fetch(`${API}/motoOrders/${id}`).then(r => r.json()),

  // Orders
  getOrders: () => fetch(`${API}/orders`).then(r => r.json()),
  createOrder: (motoId, description) => fetch(`${API}/order/${motoId}/${enc(description)}`, { method: 'POST' }).then(r => r.json()),
  updateOrderStatus: (id, status) => fetch(`${API}/orderStatus/${id}/${status}`, { method: 'PUT' }).then(r => r.json()),
  deleteOrder: (id) => fetch(`${API}/order/${id}`, { method: 'DELETE' }).then(r => r.json()),
  getOrderItems: (id) => fetch(`${API}/orderItems/${id}`).then(r => r.json()),
  addOrderItem: (orderId, partId, quantity) => fetch(`${API}/orderItem/${orderId}/${partId}/${quantity}`, { method: 'POST' }).then(r => r.json()),

  // Reports
  getLowStock: () => fetch(`${API}/reportsLowStock`).then(r => r.json()),
};

function enc(s) { return encodeURIComponent(s); }
