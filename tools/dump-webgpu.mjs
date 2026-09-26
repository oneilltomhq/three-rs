#!/usr/bin/env node
// Dump the WGSL and GPU descriptors Three's WebGPURenderer hands the driver
// for one example, the same way every rung's scout has done it by hand with
// a throwaway puppeteer script in the vendor tree. This is that script, kept.
//
// Usage:
//   node tools/dump-webgpu.mjs <example_name> [--out DIR] [--html FILE]
//
// `--html FILE` serves FILE in place of the checkout's
// `examples/<example_name>.html`, for a page that is not a three.js example
// (tools/dump-pages/, e.g. one fogged material in isolation). It is served at
// that same URL, so the page's import map and relative asset paths resolve
// against the checkout exactly as an example's do.
//
// Reads the three.js checkout at $THREE_JS_DIR (default ~/src/vendor/three.js,
// matching src/testing.rs::vendor_dir) and Chrome + puppeteer-core from that
// checkout's node_modules — nothing is installed or written there. Writes,
// under `--out` (default target/dumps/<example>/):
//
//   dump.json        modules, pipelines, layouts, bind groups, buffers,
//                     textures, samplers, passes and a flat chronological
//                     `order` log of every create/pass/submit call, exactly
//                     as the driver received them, with Three's own labels.
//   mNN_<stage>_<label>.wgsl   one file per shader module, creation order.
//   actual_full.png   the raw 800x500 frame.
//   actual.jpg         the same, through three.js' own image.js `scale()` —
//                       byte-for-byte what the grader compares.
//
// See docs/dumping.md for what a rung does with this.

import path from 'node:path';
import fs from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..');

function usage(msg) {
	if (msg) console.error(msg);
	console.error('usage: node tools/dump-webgpu.mjs <example_name> [--out DIR] [--html FILE]');
	process.exit(1);
}

function parseArgs(argv) {
	const args = argv.slice(2);
	if (args.length === 0 || args[0].startsWith('-')) usage();
	const example = args[0];
	let out = null;
	let html = null;
	for (let i = 1; i < args.length; i++) {
		if (args[i] === '--out') {
			out = args[++i];
			if (!out) usage('--out needs a DIR');
		} else if (args[i] === '--html') {
			html = args[++i];
			if (!html) usage('--html needs a FILE');
		} else {
			usage(`unrecognised argument: ${args[i]}`);
		}
	}
	return { example, out, html };
}

const { example, out, html } = parseArgs(process.argv);

const vendorDir = process.env.THREE_JS_DIR
	|| path.join(process.env.HOME, 'src/vendor/three.js');

const outDir = out ? path.resolve(out) : path.join(repoRoot, 'target/dumps', example);
const profileDir = path.join(repoRoot, 'target', '.dump-webgpu-profile');

async function importFromVendor(pkg, rel) {
	const p = path.join(vendorDir, 'node_modules', pkg, rel);
	try {
		await fs.access(p);
	} catch {
		throw new Error(
			`${pkg} not found under ${vendorDir}/node_modules — ` +
			`run "npm ci" in the vendor checkout first (see README.md)`
		);
	}
	return import(pathToFileURL(p).href);
}

// ---------------------------------------------------------------------------
// The in-page spy. Runs inside Chrome via evaluateOnNewDocument, so it has no
// access to anything in this file's scope — it is a self-contained string.
// Mirrors the wrapping described in the rung10/rung12 scout plans: every
// GPUDevice/GPUCommandEncoder/GPU*PassEncoder/GPUQueue/GPUAdapter method that
// hands the driver something durable is wrapped, and the wrapper records the
// call before delegating to the original.
// ---------------------------------------------------------------------------
const SPY_SCRIPT = String.raw`
(function () {

	if (globalThis.__dumpInjected === true) return;
	globalThis.__dumpInjected = true;

	const dump = {
		modules: [],
		bindGroupLayouts: [],
		pipelineLayouts: [],
		renderPipelines: [],
		computePipelines: [],
		buffers: [],
		textures: [],
		textureViews: [],
		samplers: [],
		bindGroups: [],
		passes: [],
		order: [],
		requestedLimits: null,
		grantedLimits: null,
	};
	globalThis.__dump = dump;

	let nextId = 0;
	const idOf = new WeakMap();
	const labelOf = new WeakMap();
	const viewSource = new WeakMap();

	function assignId(obj, label) {
		const id = nextId++;
		idOf.set(obj, id);
		labelOf.set(obj, label ?? null);
		return id;
	}

	function refId(obj) {
		if (obj == null) return null;
		if (!idOf.has(obj)) return { unknown: true, label: obj.label ?? null };
		return idOf.get(obj);
	}

	function refLabel(obj) {
		if (obj == null) return null;
		return labelOf.get(obj) ?? obj.label ?? null;
	}

	function logOrder(op, detail) {
		dump.order.push(Object.assign({ n: dump.order.length, op }, detail));
	}

	// GPUExtent3D can be either an array [width, height, depthOrArrayLayers]
	// or a dict {width, height, depthOrArrayLayers}; height/depth default to 1.
	function normalizeExtent(size) {
		if (Array.isArray(size)) {
			return { width: size[0] ?? 0, height: size[1] ?? 1, depthOrArrayLayers: size[2] ?? 1 };
		}
		return {
			width: (size && size.width) ?? 0,
			height: (size && size.height) ?? 1,
			depthOrArrayLayers: (size && size.depthOrArrayLayers) ?? 1,
		};
	}

	function decodeBufferUsage(usage) {
		const flags = {
			MAP_READ: 0x0001, MAP_WRITE: 0x0002, COPY_SRC: 0x0004, COPY_DST: 0x0008,
			INDEX: 0x0010, VERTEX: 0x0020, UNIFORM: 0x0040, STORAGE: 0x0080,
			INDIRECT: 0x0100, QUERY_RESOLVE: 0x0200,
		};
		return Object.keys(flags).filter((k) => (usage & flags[k]) !== 0);
	}

	// A descriptor argument belongs to its caller, and callers are free to keep
	// mutating it after the call returns: Three's WebGPUTextureUtils.createSampler
	// resets and reuses ONE descriptor object for every sampler it creates, so a
	// reference held until serialisation reads back the last reset state (every
	// sampler nearest/nearest/nearest, lodMaxClamp 32, no label) rather than what
	// the driver was actually asked for. Snapshot the values at call time.
	function snapshotDescriptor(descriptor) {
		if (descriptor == null || typeof descriptor !== 'object') return {};
		const out = {};
		for (const key of Object.keys(descriptor)) {
			const value = descriptor[key];
			out[key] = Array.isArray(value) ? value.slice() : value;
		}
		return out;
	}

	function decodeTextureUsage(usage) {
		const flags = {
			COPY_SRC: 0x01, COPY_DST: 0x02, TEXTURE_BINDING: 0x04,
			STORAGE_BINDING: 0x08, RENDER_ATTACHMENT: 0x10,
		};
		return Object.keys(flags).filter((k) => (usage & flags[k]) !== 0);
	}

	function decodeShaderStageVisibility(vis) {
		const flags = { VERTEX: 0x1, FRAGMENT: 0x2, COMPUTE: 0x4 };
		return Object.keys(flags).filter((k) => (vis & flags[k]) !== 0);
	}

	// Bind-group-layout entries and bind-group entries carry plain data plus,
	// for a buffer/sampler/texture-view resource, a live GPU object — resolve
	// those back to the id of the object that created them.
	function describeResource(resource) {
		if (resource == null) return null;
		if (resource instanceof GPUSampler) {
			return { sampler: refId(resource), label: refLabel(resource) };
		}
		if (resource instanceof GPUTextureView) {
			const tex = viewSource.get(resource);
			return { textureView: refId(resource), texture: tex ? refId(tex) : null, label: refLabel(resource) };
		}
		if (resource && typeof resource === 'object' && 'buffer' in resource) {
			return {
				buffer: refId(resource.buffer),
				bufferLabel: refLabel(resource.buffer),
				offset: resource.offset ?? 0,
				size: resource.size ?? null,
			};
		}
		return { unknown: String(resource) };
	}

	function describeBglEntries(entries) {
		return (entries || []).map((e) => {
			const out = { binding: e.binding, visibility: e.visibility, visibilityFlags: decodeShaderStageVisibility(e.visibility) };
			if (e.buffer) out.buffer = e.buffer;
			if (e.sampler) out.sampler = e.sampler;
			if (e.texture) out.texture = e.texture;
			if (e.storageTexture) out.storageTexture = e.storageTexture;
			if (e.externalTexture) out.externalTexture = e.externalTexture;
			return out;
		});
	}

	function describeAttachment(a) {
		if (!a) return null;
		const out = Object.assign({}, a);
		if (a.view) {
			const tex = viewSource.get(a.view);
			out.view = { textureView: refId(a.view), texture: tex ? refId(tex) : null, label: refLabel(a.view) };
		}
		if (a.resolveTarget) {
			const tex = viewSource.get(a.resolveTarget);
			out.resolveTarget = { textureView: refId(a.resolveTarget), texture: tex ? refId(tex) : null };
		}
		return out;
	}

	function describeRenderPassDesc(desc) {
		return {
			label: desc.label ?? null,
			colorAttachments: (desc.colorAttachments || []).map(describeAttachment),
			depthStencilAttachment: describeAttachment(desc.depthStencilAttachment),
		};
	}

	function describeComputePassDesc(desc) {
		return { label: (desc && desc.label) ?? null };
	}

	function shaderStageOf(code) {
		if (/@vertex/.test(code) && /@fragment/.test(code)) return 'vertex_fragment';
		if (/@vertex/.test(code)) return 'vertex';
		if (/@fragment/.test(code)) return 'fragment';
		if (/@compute/.test(code)) return 'compute';
		return 'unknown';
	}

	// ---- GPUTexture / GPUCanvasContext ------------------------------------
	//
	// Wrapped once at the prototype level, not per created texture, so a
	// swap-chain texture from context.getCurrentTexture() (never passed
	// through device.createTexture) still gets an id the first time
	// something views it.

	if (typeof GPUTexture !== 'undefined' && !GPUTexture.prototype.__dumpWrapped) {
		const origCreateView = GPUTexture.prototype.createView;
		GPUTexture.prototype.createView = function (descriptor) {
			const view = origCreateView.call(this, descriptor);
			if (!idOf.has(this)) assignId(this, this.label || null);
			viewSource.set(view, this);
			const id = assignId(view, (descriptor && descriptor.label) ?? null);
			dump.textureViews.push({ id, label: (descriptor && descriptor.label) ?? null, texture: refId(this), desc: descriptor || {} });
			return view;
		};
		GPUTexture.prototype.__dumpWrapped = true;
	}

	if (typeof GPUCanvasContext !== 'undefined' && !GPUCanvasContext.prototype.__dumpWrapped) {
		const origGetCurrentTexture = GPUCanvasContext.prototype.getCurrentTexture;
		GPUCanvasContext.prototype.getCurrentTexture = function () {
			const texture = origGetCurrentTexture.call(this);
			if (!idOf.has(texture)) assignId(texture, '<canvas>');
			return texture;
		};
		GPUCanvasContext.prototype.__dumpWrapped = true;
	}

	// ---- GPUAdapter -----------------------------------------------------

	const origRequestDevice = GPUAdapter.prototype.requestDevice;
	GPUAdapter.prototype.requestDevice = async function (descriptor) {
		if (descriptor && descriptor.requiredLimits) {
			dump.requestedLimits = descriptor.requiredLimits;
		}
		const device = await origRequestDevice.call(this, descriptor);
		try {
			const limits = {};
			for (const k in device.limits) limits[k] = device.limits[k];
			dump.grantedLimits = limits;
		} catch (e) { /* ignore */ }
		instrumentDevice(device);
		return device;
	};

	// ---- GPUDevice --------------------------------------------------------

	function instrumentDevice(device) {

		if (device.__dumpInstrumented) return;
		device.__dumpInstrumented = true;

		const origCreateShaderModule = device.createShaderModule.bind(device);
		device.createShaderModule = function (descriptor) {
			const module = origCreateShaderModule(descriptor);
			const id = assignId(module, descriptor.label ?? null);
			dump.modules.push({
				id,
				label: descriptor.label ?? null,
				stage: shaderStageOf(descriptor.code || ''),
				code: descriptor.code || '',
			});
			logOrder('createShaderModule', { id, label: descriptor.label ?? null });
			return module;
		};

		const origCreateBindGroupLayout = device.createBindGroupLayout.bind(device);
		device.createBindGroupLayout = function (descriptor) {
			const bgl = origCreateBindGroupLayout(descriptor);
			const id = assignId(bgl, descriptor.label ?? null);
			dump.bindGroupLayouts.push({ id, label: descriptor.label ?? null, entries: describeBglEntries(descriptor.entries) });
			logOrder('createBindGroupLayout', { id, label: descriptor.label ?? null });
			return bgl;
		};

		const origCreatePipelineLayout = device.createPipelineLayout.bind(device);
		device.createPipelineLayout = function (descriptor) {
			const layout = origCreatePipelineLayout(descriptor);
			const id = assignId(layout, descriptor.label ?? null);
			dump.pipelineLayouts.push({
				id,
				label: descriptor.label ?? null,
				bindGroupLayouts: (descriptor.bindGroupLayouts || []).map(refId),
			});
			logOrder('createPipelineLayout', { id, label: descriptor.label ?? null });
			return layout;
		};

		function sanitizeRenderDescriptor(descriptor) {
			const out = JSON.parse(JSON.stringify(descriptor, (k, v) => (k === 'module' ? undefined : v)));
			out.vertex.module = refLabel(descriptor.vertex.module);
			if (descriptor.fragment) out.fragment.module = refLabel(descriptor.fragment.module);
			out.layout = descriptor.layout === 'auto' ? 'auto' : refId(descriptor.layout);
			return out;
		}

		function sanitizeComputeDescriptor(descriptor) {
			const out = JSON.parse(JSON.stringify(descriptor, (k, v) => (k === 'module' ? undefined : v)));
			out.compute.module = refLabel(descriptor.compute.module);
			out.layout = descriptor.layout === 'auto' ? 'auto' : refId(descriptor.layout);
			return out;
		}

		function wrapCreateRenderPipeline(name, async) {
			const orig = device[name].bind(device);
			device[name] = function (descriptor) {
				const result = orig(descriptor);
				const record = (pipeline) => {
					const id = assignId(pipeline, descriptor.label ?? null);
					dump.renderPipelines.push({ id, label: descriptor.label ?? null, async, desc: sanitizeRenderDescriptor(descriptor) });
					logOrder(name, { id, label: descriptor.label ?? null });
					return pipeline;
				};
				return async ? result.then(record) : record(result);
			};
		}
		wrapCreateRenderPipeline('createRenderPipeline', false);
		wrapCreateRenderPipeline('createRenderPipelineAsync', true);

		function wrapCreateComputePipeline(name, async) {
			const orig = device[name].bind(device);
			device[name] = function (descriptor) {
				const result = orig(descriptor);
				const record = (pipeline) => {
					const id = assignId(pipeline, descriptor.label ?? null);
					dump.computePipelines.push({ id, label: descriptor.label ?? null, async, desc: sanitizeComputeDescriptor(descriptor) });
					logOrder(name, { id, label: descriptor.label ?? null });
					return pipeline;
				};
				return async ? result.then(record) : record(result);
			};
		}
		wrapCreateComputePipeline('createComputePipeline', false);
		wrapCreateComputePipeline('createComputePipelineAsync', true);

		const origCreateBuffer = device.createBuffer.bind(device);
		device.createBuffer = function (descriptor) {
			const buffer = origCreateBuffer(descriptor);
			const id = assignId(buffer, descriptor.label ?? null);
			dump.buffers.push({
				id,
				label: descriptor.label ?? null,
				size: descriptor.size,
				usage: descriptor.usage,
				usageFlags: decodeBufferUsage(descriptor.usage),
				mappedAtCreation: !!descriptor.mappedAtCreation,
			});
			logOrder('createBuffer', { id, label: descriptor.label ?? null, size: descriptor.size });
			return buffer;
		};

		const origCreateTexture = device.createTexture.bind(device);
		device.createTexture = function (descriptor) {
			const texture = origCreateTexture(descriptor);
			const id = assignId(texture, descriptor.label ?? null);
			dump.textures.push({
				id,
				label: descriptor.label ?? null,
				desc: {
					size: descriptor.size,
					mipLevelCount: descriptor.mipLevelCount ?? 1,
					sampleCount: descriptor.sampleCount ?? 1,
					dimension: descriptor.dimension ?? '2d',
					format: descriptor.format,
					usage: descriptor.usage,
					usageFlags: decodeTextureUsage(descriptor.usage),
					viewFormats: descriptor.viewFormats ?? [],
				},
			});
			logOrder('createTexture', { id, label: descriptor.label ?? null, format: descriptor.format });
			return texture;
		};

		const origCreateSampler = device.createSampler.bind(device);
		device.createSampler = function (descriptor) {
			const sampler = origCreateSampler(descriptor || {});
			const desc = snapshotDescriptor(descriptor);
			const id = assignId(sampler, desc.label ?? null);
			dump.samplers.push({ id, label: desc.label ?? null, desc });
			logOrder('createSampler', { id, label: desc.label ?? null });
			return sampler;
		};

		const origCreateBindGroup = device.createBindGroup.bind(device);
		device.createBindGroup = function (descriptor) {
			const bindGroup = origCreateBindGroup(descriptor);
			const id = assignId(bindGroup, descriptor.label ?? null);
			dump.bindGroups.push({
				id,
				label: descriptor.label ?? null,
				layout: refId(descriptor.layout),
				entries: (descriptor.entries || []).map((e) => ({ binding: e.binding, resource: describeResource(e.resource) })),
			});
			logOrder('createBindGroup', { id, label: descriptor.label ?? null });
			return bindGroup;
		};

		const origCreateCommandEncoder = device.createCommandEncoder.bind(device);
		device.createCommandEncoder = function (descriptor) {
			const encoder = origCreateCommandEncoder(descriptor);
			instrumentCommandEncoder(encoder, (descriptor && descriptor.label) ?? null);
			return encoder;
		};

		instrumentQueue(device.queue);
	}

	function instrumentPassEncoder(encoder, passRecord) {
		const cmd = (op, detail) => {
			const entry = Object.assign({ op }, detail);
			passRecord.cmds.push(entry);
			logOrder(op, Object.assign({ pass: passRecord.id }, detail));
		};

		if (encoder.setPipeline) {
			const orig = encoder.setPipeline.bind(encoder);
			encoder.setPipeline = function (pipeline) {
				cmd('setPipeline', { pipeline: refId(pipeline), label: refLabel(pipeline) });
				return orig(pipeline);
			};
		}
		if (encoder.setBindGroup) {
			const orig = encoder.setBindGroup.bind(encoder);
			encoder.setBindGroup = function (index, bindGroup, ...rest) {
				cmd('setBindGroup', { index, bindGroup: refId(bindGroup), label: refLabel(bindGroup) });
				return orig(index, bindGroup, ...rest);
			};
		}
		if (encoder.setVertexBuffer) {
			const orig = encoder.setVertexBuffer.bind(encoder);
			encoder.setVertexBuffer = function (slot, buffer, ...rest) {
				cmd('setVertexBuffer', { slot, buffer: refId(buffer), label: refLabel(buffer) });
				return orig(slot, buffer, ...rest);
			};
		}
		if (encoder.setIndexBuffer) {
			const orig = encoder.setIndexBuffer.bind(encoder);
			encoder.setIndexBuffer = function (buffer, format, ...rest) {
				cmd('setIndexBuffer', { buffer: refId(buffer), label: refLabel(buffer), format });
				return orig(buffer, format, ...rest);
			};
		}
		if (encoder.setViewport) {
			const orig = encoder.setViewport.bind(encoder);
			encoder.setViewport = function (x, y, width, height, minDepth, maxDepth) {
				cmd('setViewport', { x, y, width, height, minDepth, maxDepth });
				return orig(x, y, width, height, minDepth, maxDepth);
			};
		}
		if (encoder.setScissorRect) {
			const orig = encoder.setScissorRect.bind(encoder);
			encoder.setScissorRect = function (x, y, width, height) {
				cmd('setScissorRect', { x, y, width, height });
				return orig(x, y, width, height);
			};
		}
		if (encoder.setBlendConstant) {
			const orig = encoder.setBlendConstant.bind(encoder);
			encoder.setBlendConstant = function (color) {
				cmd('setBlendConstant', { color });
				return orig(color);
			};
		}
		if (encoder.setStencilReference) {
			const orig = encoder.setStencilReference.bind(encoder);
			encoder.setStencilReference = function (reference) {
				cmd('setStencilReference', { reference });
				return orig(reference);
			};
		}
		if (encoder.draw) {
			const orig = encoder.draw.bind(encoder);
			encoder.draw = function (vertexCount, instanceCount, firstVertex, firstInstance) {
				cmd('draw', { vertexCount, instanceCount: instanceCount ?? 1, firstVertex: firstVertex ?? 0, firstInstance: firstInstance ?? 0 });
				return orig(vertexCount, instanceCount, firstVertex, firstInstance);
			};
		}
		if (encoder.drawIndexed) {
			const orig = encoder.drawIndexed.bind(encoder);
			encoder.drawIndexed = function (indexCount, instanceCount, firstIndex, baseVertex, firstInstance) {
				cmd('drawIndexed', {
					indexCount, instanceCount: instanceCount ?? 1, firstIndex: firstIndex ?? 0,
					baseVertex: baseVertex ?? 0, firstInstance: firstInstance ?? 0,
				});
				return orig(indexCount, instanceCount, firstIndex, baseVertex, firstInstance);
			};
		}
		// The indirect forms (issue #167): the arguments live in a GPU buffer,
		// so the log names the buffer and the offset, not the counts.
		for (const name of ['drawIndirect', 'drawIndexedIndirect', 'dispatchWorkgroupsIndirect']) {
			if (encoder[name]) {
				const orig = encoder[name].bind(encoder);
				encoder[name] = function (buffer, offset) {
					cmd(name, { buffer: refId(buffer), label: refLabel(buffer), offset });
					return orig(buffer, offset);
				};
			}
		}
		if (encoder.dispatchWorkgroups) {
			const orig = encoder.dispatchWorkgroups.bind(encoder);
			encoder.dispatchWorkgroups = function (x, y, z) {
				cmd('dispatchWorkgroups', { x, y: y ?? 1, z: z ?? 1 });
				return orig(x, y, z);
			};
		}
		const origEnd = encoder.end.bind(encoder);
		encoder.end = function () {
			cmd('end', {});
			return origEnd();
		};
	}

	function instrumentCommandEncoder(encoder, label) {
		const origBeginRenderPass = encoder.beginRenderPass.bind(encoder);
		encoder.beginRenderPass = function (descriptor) {
			const pass = origBeginRenderPass(descriptor);
			const passRecord = { id: dump.passes.length, kind: 'render', encoderLabel: label, desc: describeRenderPassDesc(descriptor), cmds: [] };
			dump.passes.push(passRecord);
			logOrder('beginRenderPass', { pass: passRecord.id, label: descriptor.label ?? null });
			instrumentPassEncoder(pass, passRecord);
			return pass;
		};

		const origBeginComputePass = encoder.beginComputePass.bind(encoder);
		encoder.beginComputePass = function (descriptor) {
			const pass = origBeginComputePass(descriptor);
			const passRecord = { id: dump.passes.length, kind: 'compute', encoderLabel: label, desc: describeComputePassDesc(descriptor), cmds: [] };
			dump.passes.push(passRecord);
			logOrder('beginComputePass', { pass: passRecord.id, label: (descriptor && descriptor.label) ?? null });
			instrumentPassEncoder(pass, passRecord);
			return pass;
		};

		const origFinish = encoder.finish.bind(encoder);
		encoder.finish = function (descriptor) {
			const buf = origFinish(descriptor);
			logOrder('commandEncoder.finish', { encoderLabel: label });
			return buf;
		};
	}

	function instrumentQueue(queue) {
		if (queue.__dumpInstrumented) return;
		queue.__dumpInstrumented = true;

		const origSubmit = queue.submit.bind(queue);
		queue.submit = function (commandBuffers) {
			logOrder('queue.submit', { count: commandBuffers.length });
			return origSubmit(commandBuffers);
		};

		const origWriteBuffer = queue.writeBuffer.bind(queue);
		queue.writeBuffer = function (buffer, bufferOffset, data, dataOffset, size) {
			logOrder('queue.writeBuffer', {
				buffer: refId(buffer), label: refLabel(buffer), bufferOffset,
				byteLength: (data && data.byteLength) ?? null,
			});
			return origWriteBuffer(buffer, bufferOffset, data, dataOffset, size);
		};

		const origWriteTexture = queue.writeTexture.bind(queue);
		queue.writeTexture = function (destination, data, dataLayout, size) {
			logOrder('queue.writeTexture', {
				texture: refId(destination.texture), label: refLabel(destination.texture), size: normalizeExtent(size),
			});
			return origWriteTexture(destination, data, dataLayout, size);
		};
	}

}());
`;

// ---------------------------------------------------------------------------

async function main() {
	const { default: puppeteerFull } = await importFromVendor('puppeteer', 'lib/puppeteer/puppeteer.js');
	const { default: puppeteer } = await importFromVendor('puppeteer-core', 'lib/puppeteer/puppeteer-core.js');

	await fs.mkdir(outDir, { recursive: true });
	await fs.mkdir(path.dirname(profileDir), { recursive: true });

	const executablePath = await puppeteerFull.executablePath();

	// Serve the vendor checkout exactly as test/e2e/puppeteer.js does, but on
	// our own port so a concurrent grader run never collides with us.
	const serverMod = await import(pathToFileURL(path.join(vendorDir, 'utils/server.js')).href);
	const server = serverMod.createServer({ root: vendorDir });
	const port = await new Promise((resolve, reject) => {
		server.once('error', reject);
		server.listen(0, () => resolve(server.address().port));
	});

	const cleanPage = await fs.readFile(path.join(vendorDir, 'test/e2e/clean-page.js'), 'utf8');
	const deterministicInjection = await fs.readFile(path.join(vendorDir, 'test/e2e/deterministic-injection.js'), 'utf8');

	// Same rewrite `test/e2e/puppeteer.js` applies to every build file: route
	// Math.random through the seeded one deterministic-injection.js installs,
	// and disable timestamp queries (they crash the software Vulkan path).
	const buildInjection = (code) => (deterministicInjection + '\n' + SPY_SCRIPT + '\n' + code)
		.replace(/Math\.random\(\) \* 0xffffffff/g, 'Math._random() * 0xffffffff')
		.replace(
			/this\.trackTimestamp\s*=\s*\(\s*parameters\.trackTimestamp\s*===\s*true\s*\);/,
			"Object.defineProperty(this, 'trackTimestamp', { get: () => false, set: () => {} });"
		);


	const buildNames = ['three.core.js', 'three.module.js', 'three.webgpu.js'];
	const builds = {};
	for (const name of buildNames) {
		builds[name] = buildInjection(await fs.readFile(path.join(vendorDir, 'build', name), 'utf8'));
	}

	const flags = [
		'--hide-scrollbars',
		'--enable-unsafe-webgpu',
		'--enable-features=Vulkan',
		'--use-angle=vulkan', // grader-flags.patch: no --disable-vulkan-surface
		'--ignore-gpu-blocklist',
		'--disable-gpu-driver-bug-workarounds',
		'--disable-gpu-watchdog',
		'--no-sandbox',
	];

	const width = 400;
	const height = 250;
	const viewScale = 2;

	const browser = await puppeteer.launch({
		executablePath,
		headless: process.env.VISIBLE ? false : 'new',
		env: { ...process.env },
		args: flags,
		defaultViewport: { width: width * viewScale, height: height * viewScale },
		handleSIGINT: false,
		protocolTimeout: 0,
		userDataDir: profileDir,
	});

	try {
		const page = await browser.newPage();
		const client = await page.createCDPSession();
		await client.send('Input.setIgnoreInputEvents', { ignore: true });

		await page.evaluateOnNewDocument(deterministicInjection + '\n' + SPY_SCRIPT);
		await page.setRequestInterception(true);

		const errors = [];
		page.on('pageerror', (error) => errors.push(String(error.message || error)));
		page.on('console', (msg) => {
			if (msg.type() === 'error') errors.push(msg.text());
		});

		const pageBody = html ? await fs.readFile(path.resolve(html), 'utf8') : null;

		page.on('request', async (request) => {
			const url = request.url();
			if (pageBody !== null && url === `http://localhost:${port}/examples/${example}.html`) {
				await request.respond({ status: 200, contentType: 'text/html; charset=utf-8', body: pageBody });
				return;
			}
			for (const build in builds) {
				if (url === `http://localhost:${port}/build/${build}`) {
					await request.respond({ status: 200, contentType: 'application/javascript; charset=utf-8', body: builds[build] });
					return;
				}
			}
			await request.continue();
		});

		await page.goto(`http://localhost:${port}/examples/${example}.html`, {
			waitUntil: 'networkidle0',
			timeout: 5 * 60 * 1000,
		});

		await page.evaluate(cleanPage);

		await page.waitForNetworkIdle({ timeout: 5 * 60 * 1000, idleTime: 2000 });
		await page.waitForFunction(() => window._videosReady(), { polling: 100, timeout: 5000 });

		// Same handshake as checkFile(): flip _renderStarted, wait for the
		// (deterministic, single) RAF to fire and _renderFinished to flip back.
		await page.evaluate(async () => {
			window._renderStarted = true;
			await new Promise((resolve, reject) => {
				const start = performance._now();
				const loop = setInterval(() => {
					if (performance._now() - start > 5000) {
						clearInterval(loop);
						reject(new Error('render timeout exceeded'));
					} else if (window._renderFinished) {
						clearInterval(loop);
						resolve();
					}
				}, 100);
			});
		});

		if (errors.length) {
			console.error(`${example}: page reported error(s):\n${errors.join('\n')}`);
		}

		// One more RAF's worth of settling so the frame the dump describes and
		// the frame the screenshot shows are the same one.
		await new Promise((resolve) => setTimeout(resolve, 50));

		const pngBuffer = await page.screenshot({ type: 'png' });
		await fs.writeFile(path.join(outDir, 'actual_full.png'), pngBuffer);

		// Three's own downscale + JPEG encode, byte-for-byte what the grader
		// compares — image.js has no other dependency on the e2e harness.
		const { Image } = await import(pathToFileURL(path.join(vendorDir, 'test/e2e/image.js')).href);
		const scaled = (await Image.read(pngBuffer)).scale(1 / viewScale);
		await scaled.write(path.join(outDir, 'actual.jpg'), 95);

		const dump = await page.evaluate(() => window.__dump);

		// Split the modules out to their own files, named by creation order,
		// detected stage and Three's own label; keep only ids/labels in dump.json.
		const digits = String(dump.modules.length - 1).length;
		const moduleFiles = [];
		for (let i = 0; i < dump.modules.length; i++) {
			const m = dump.modules[i];
			const idx = String(i).padStart(Math.max(2, digits), '0');
			const label = (m.label || 'unlabelled').replace(/[^A-Za-z0-9_.-]+/g, '_');
			const file = `m${idx}_${m.stage}_${label}.wgsl`;
			await fs.writeFile(path.join(outDir, file), m.code);
			moduleFiles.push({ id: m.id, label: m.label, stage: m.stage, file });
		}
		dump.modules = dump.modules.map((m, i) => ({ id: m.id, label: m.label, stage: m.stage, file: moduleFiles[i].file }));

		await fs.writeFile(path.join(outDir, 'dump.json'), JSON.stringify(dump, null, 2));

		console.log(`${example}: wrote ${dump.modules.length} module(s), ` +
			`${dump.renderPipelines.length} render + ${dump.computePipelines.length} compute pipeline(s), ` +
			`${dump.passes.length} pass(es) to ${outDir}`);
	} finally {
		await browser.close();
		server.close();
	}
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
