const PI: f32 = 3.1415926535897932384626433832795;


@group(0)
@binding(0)
var dst: texture_storage_2d<rgba32float, write>;


fn RadicalInverse_VdC(in_bits : u32) -> f32
{
    var bits = (in_bits << 16u) | (in_bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10; // / 0x100000000
}
// ----------------------------------------------------------------------------
fn Hammersley(i : u32, N : u32) -> vec2<f32>
{
    return vec2<f32>(f32(i)/f32(N), RadicalInverse_VdC(i));
}  


fn GeometrySchlickGGX( NdotV : f32, roughness : f32) -> f32
{
    var  a = roughness;
    var  k = (a * a) / 2.0;

    var  nom   = NdotV;
    var  denom = NdotV * (1.0 - k) + k;

    return nom / denom;
}
// ----------------------------------------------------------------------------
fn GeometrySmith( N : vec3<f32>, V : vec3<f32>, L : vec3<f32>, roughness : f32) -> f32
{
    var NdotV = max(dot(N, V), 0.0);
    var NdotL = max(dot(N, L), 0.0);
    var ggx2 = GeometrySchlickGGX(NdotV, roughness);
    var ggx1 = GeometrySchlickGGX(NdotL, roughness);

    return ggx1 * ggx2;
} 

fn ImportanceSampleGGX(Xi : vec2<f32>, N : vec3<f32>, roughness : f32) -> vec3<f32>
{
    var a = roughness*roughness;
	
    var phi = 2.0 * PI * Xi.x;
    var cosTheta = sqrt((1.0 - Xi.y) / (1.0 + (a*a - 1.0) * Xi.y));
    var sinTheta = sqrt(1.0 - cosTheta*cosTheta);
	
    // from spherical coordinates to cartesian coordinates
    var H : vec3<f32>;
    H.x = cos(phi) * sinTheta;
    H.y = sin(phi) * sinTheta;
    H.z = cosTheta;
	
    // from tangent-space vector to world-space sample vector
    var up = vec3(1.0, 0.0, 0.0);

    if( abs(N.z) < 0.999 ){
        up = vec3(0.0, 0.0, 1.0);
    }

    var tangent   = normalize(cross(up, N));
    var bitangent = cross(N, tangent);
	
    var sampleVec = tangent * H.x + bitangent * H.y + N * H.z;
    return normalize(sampleVec);
}  

fn IntegrateBRDF( NdotV : f32, roughness : f32) -> vec2<f32>
{
    var V : vec3<f32> = vec3<f32>(0.0);
    V.x = sqrt(1.0 - NdotV*NdotV);
    V.y = 0.0;
    V.z = NdotV;

    var A = 0.0;
    var B = 0.0;

    var N = vec3<f32>(0.0, 0.0, 1.0);

    let SAMPLE_COUNT = 1024u;
    for(var i = 0u; i < SAMPLE_COUNT; i++)
    {
        var Xi = Hammersley(i, SAMPLE_COUNT);
        var H  = ImportanceSampleGGX(Xi, N, roughness);
        var L  = normalize(2.0 * dot(V, H) * H - V);

        var NdotL = max(L.z, 0.0);
        var NdotH = max(H.z, 0.0);
        var VdotH = max(dot(V, H), 0.0);

        if(NdotL > 0.0)
        {
            var G = GeometrySmith(N, V, L, roughness);
            var G_Vis = (G * VdotH) / (NdotH * NdotV);
            var Fc = pow(1.0 - VdotH, 5.0);

            A += (1.0 - Fc) * G_Vis;
            B += Fc * G_Vis;
        }
    }
    A /= f32(SAMPLE_COUNT);
    B /= f32(SAMPLE_COUNT);
    return vec2<f32>(A, B);
}


@compute
@workgroup_size(16, 16, 1)
fn main(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {

    var x = f32(gid.x) + 1.0;
    var coords = vec2<f32>( x / 512.0, f32(gid.y) / 512.0);
    var integratedBRDF = IntegrateBRDF(coords.x, coords.y);
    textureStore(dst, gid.xy, vec4( integratedBRDF, 0.0, 1.0));
}
