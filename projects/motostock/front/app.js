// MotoStock - Frontend Logic
const API = 'http://localhost:8081';

// Navigation
function showSection(id) {
    document.querySelectorAll('main > section').forEach(s => s.classList.add('hidden'));
    document.getElementById(id)?.classList.remove('hidden');
    if (id === 'orders') loadMotosSelect();
}

// Modals
function showModal(id) { document.getElementById(id)?.showModal(); }
function closeModal(id) { document.getElementById(id)?.close(); }

// HTMX response transformers
document.body.addEventListener('htmx:beforeSwap', function(evt) {
    const target = evt.detail.target.id;
    const data = JSON.parse(evt.detail.xhr.responseText);

    if (target === 'parts-table') {
        evt.detail.serverResponse = renderPartsTable(data);
    } else if (target === 'motos-table') {
        evt.detail.serverResponse = renderMotosTable(data);
    } else if (target === 'orders-table') {
        evt.detail.serverResponse = renderOrdersTable(data);
    } else if (target === 'inventory-stats') {
        evt.detail.serverResponse = renderInventoryStats(data);
    } else if (target === 'low-stock-count') {
        evt.detail.serverResponse = renderLowStockCount(data);
    } else if (target === 'monthly-stats') {
        evt.detail.serverResponse = renderMonthlyStats(data);
    } else if (target === 'low-stock-table') {
        evt.detail.serverResponse = renderLowStockTable(data);
    }
});

// Renderers
function renderPartsTable(parts) {
    if (!parts?.length) return '<tr><td colspan="6">No hay repuestos</td></tr>';
    return parts.map(p => `
        <tr>
            <td>${p.code}</td>
            <td>${p.name}</td>
            <td>${p.brand || '-'}</td>
            <td>$${p.price.toFixed(2)}</td>
            <td class="${p.stock < p.min_stock ? 'low-stock' : ''}">${p.stock} / ${p.min_stock}</td>
            <td>
                <button class="btn-sm btn-danger" onclick="deletePart(${p.id})">🗑️</button>
            </td>
        </tr>
    `).join('');
}

function renderMotosTable(motos) {
    if (!motos?.length) return '<tr><td colspan="7">No hay motos</td></tr>';
    return motos.map(m => `
        <tr>
            <td><strong>${m.plate}</strong></td>
            <td>${m.brand}</td>
            <td>${m.model}</td>
            <td>${m.year || '-'}</td>
            <td>${m.owner_name}</td>
            <td>${m.owner_phone || '-'}</td>
            <td>
                <button class="btn-sm" onclick="viewMotoOrders(${m.id})">📋</button>
                <button class="btn-sm btn-danger" onclick="deleteMoto(${m.id})">🗑️</button>
            </td>
        </tr>
    `).join('');
}

function renderOrdersTable(orders) {
    if (!orders?.length) return '<tr><td colspan="7">No hay órdenes</td></tr>';
    return orders.map(o => `
        <tr>
            <td>#${o.id}</td>
            <td>${o.plate} - ${o.brand} ${o.model}</td>
            <td>${o.description}</td>
            <td><span class="badge badge-${o.status}">${o.status}</span></td>
            <td>${new Date(o.created_at).toLocaleDateString()}</td>
            <td id="order-total-${o.id}">-</td>
            <td>
                <button class="btn-sm" onclick="viewOrderItems(${o.id})">📦</button>
                <button class="btn-sm btn-success" onclick="changeOrderStatus(${o.id})">✓</button>
                <button class="btn-sm btn-danger" onclick="deleteOrder(${o.id})">🗑️</button>
            </td>
        </tr>
    `).join('');
}

function renderInventoryStats(data) {
    return `
        <div class="stat-value">$${(data.total_value || 0).toFixed(2)}</div>
        <div class="stat-label">${data.total_parts} productos, ${data.total_units} unidades</div>
    `;
}

function renderLowStockCount(data) {
    const count = Array.isArray(data) ? data.length : 0;
    return `
        <div class="stat-value ${count > 0 ? 'low-stock' : ''}">${count}</div>
        <div class="stat-label">productos bajo mínimo</div>
    `;
}

function renderMonthlyStats(data) {
    return `
        <div class="stat-value">${data.orders || 0}</div>
        <div class="stat-label">órdenes - $${(data.revenue || 0).toFixed(2)}</div>
    `;
}

function renderLowStockTable(data) {
    if (!data?.length) return '<tr><td colspan="5">Sin alertas de stock</td></tr>';
    return data.map(p => `
        <tr>
            <td>${p.code}</td>
            <td>${p.name}</td>
            <td class="low-stock">${p.stock}</td>
            <td>${p.min_stock}</td>
            <td><strong>${p.to_order}</strong></td>
        </tr>
    `).join('');
}

// CRUD Operations
async function createPart(e) {
    e.preventDefault();
    const f = new FormData(e.target);
    const url = `${API}/part/${enc(f.get('code'))}/${enc(f.get('name'))}/${enc(f.get('brand') || 'N/A')}/${f.get('price')}/${f.get('stock') || 0}/${f.get('min_stock') || 5}`;
    await fetch(url, { method: 'POST' });
    closeModal('part-modal');
    e.target.reset();
    htmx.trigger(document.body, 'partsChanged');
}

async function deletePart(id) {
    if (!confirm('¿Eliminar repuesto?')) return;
    await fetch(`${API}/part/${id}`, { method: 'DELETE' });
    htmx.trigger(document.body, 'partsChanged');
}

async function createMoto(e) {
    e.preventDefault();
    const f = new FormData(e.target);
    const url = `${API}/moto/${enc(f.get('plate'))}/${enc(f.get('brand'))}/${enc(f.get('model'))}/${f.get('year')}/${enc(f.get('owner_name'))}/${enc(f.get('owner_phone') || 'N/A')}`;
    await fetch(url, { method: 'POST' });
    closeModal('moto-modal');
    e.target.reset();
    htmx.trigger(document.body, 'motosChanged');
}

async function deleteMoto(id) {
    if (!confirm('¿Eliminar moto?')) return;
    await fetch(`${API}/moto/${id}`, { method: 'DELETE' });
    htmx.trigger(document.body, 'motosChanged');
}

async function loadMotosSelect() {
    const res = await fetch(`${API}/motos`);
    const motos = await res.json();
    const sel = document.getElementById('moto-select');
    sel.innerHTML = '<option value="">Seleccionar moto...</option>' +
        motos.map(m => `<option value="${m.id}">${m.plate} - ${m.brand} ${m.model}</option>`).join('');
}

async function createOrder(e) {
    e.preventDefault();
    const f = new FormData(e.target);
    const url = `${API}/order/${f.get('moto_id')}/${enc(f.get('description'))}`;
    await fetch(url, { method: 'POST' });
    closeModal('order-modal');
    e.target.reset();
    htmx.trigger(document.body, 'ordersChanged');
}

async function changeOrderStatus(id) {
    const statuses = ['pending', 'in_progress', 'completed'];
    const current = prompt('Nuevo estado (pending/in_progress/completed):');
    if (!current || !statuses.includes(current)) return;
    await fetch(`${API}/orderStatus/${id}/${current}`, { method: 'PUT' });
    htmx.trigger(document.body, 'ordersChanged');
}

async function deleteOrder(id) {
    if (!confirm('¿Eliminar orden?')) return;
    await fetch(`${API}/order/${id}`, { method: 'DELETE' });
    htmx.trigger(document.body, 'ordersChanged');
}

async function viewOrderItems(id) {
    const res = await fetch(`${API}/orderItems/${id}`);
    const items = await res.json();
    alert(items.length ? items.map(i => `${i.quantity}x ${i.name} - $${i.unit_price}`).join('\n') : 'Sin items');
}

async function viewMotoOrders(id) {
    const res = await fetch(`${API}/motoOrders/${id}`);
    const orders = await res.json();
    alert(orders.length ? orders.map(o => `#${o.id}: ${o.description} (${o.status})`).join('\n') : 'Sin órdenes');
}

// Helpers
function enc(s) { return encodeURIComponent(s); }

// Init
document.addEventListener('DOMContentLoaded', () => {
    showSection('dashboard');
});
